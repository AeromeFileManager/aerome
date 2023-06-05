/*
 * Copyright (c) 2023 Jesse Tuchsen
 *
 * This file is part of Aerome.
 *
 * Aerome is free software: you can redistribute it and/or modify it under the terms of the GNU
 * General Public License as published by the Free Software Foundation, version 3 of the License.
 *
 * Aerome is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even
 * the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General
 * Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with Aerome. If not, see
 * <https://www.gnu.org/licenses/>.
 */

use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use std::fs::{self,DirEntry};
use std::process::{Command,Stdio};
use xdg_mime::{SharedMimeInfo, Guess};
use wry::application::event_loop::EventLoopProxy;
use url::Url;
use tokio::runtime::Runtime;
use crate::{Icons,Thumbnails,UserEvent,Options,Folder,FolderListing,FolderListingType,FileMetadata,Sort};
use std::ffi::OsStr;
use std::cmp::Ordering;
use notify::{RecursiveMode,Watcher,RecommendedWatcher};
use notify_debouncer_mini::{new_debouncer,Debouncer,DebounceEventResult};
use std::time::{Duration,SystemTime,UNIX_EPOCH};

#[derive(Clone)]
pub struct Location {
    current: Arc<Mutex<Folder>>,
    debouncer: Arc<Mutex<Debouncer<RecommendedWatcher>>>,
    mime_db: Arc<SharedMimeInfo>,
    icons: Icons,
    options: Arc<Mutex<Options>>,
    thumbnails: Thumbnails,
    proxy: EventLoopProxy<UserEvent>,
}

impl Location {
    pub fn new(
        current: &Path,
        mime_db: SharedMimeInfo,
        proxy: EventLoopProxy<UserEvent>,
        thumbnails: Thumbnails,
        icons: Icons) -> Self
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let debouncer = new_debouncer(Duration::from_millis(100), None, tx).unwrap();
        let current = {
            Self::get_folder(
                current, &Options::default(), &mime_db, &thumbnails, &icons)
        };

        let options = Options::default();
        let location = Self {
            current: Arc::new(Mutex::new(current)),
            icons,
            debouncer: Arc::new(Mutex::new(debouncer)),
            mime_db: Arc::new(mime_db),
            options: Arc::new(Mutex::new(options)),
            proxy,
            thumbnails
        };
        let this = location.clone();

        std::thread::spawn(move || {
            for result in rx {
                let folder = this.update(&this.current_path(), &this.current_options());
                this.proxy.send_event(UserEvent::UpdateFolder {
                    folder,
                    script_result: None
                });
            }
        });

        location
    }

    pub fn current_folder(&self) -> Folder {
        self.current.lock().unwrap().clone()
    }

    pub fn current_options(&self) -> Options {
        self.options.lock().unwrap().clone()
    }

    pub fn current_path(&self) -> PathBuf {
        self.current.lock().unwrap().path.clone()
    }

    pub fn update(&self, path: &Path, options: &Options) -> Folder {
        let folder = Self::get_folder(path, options, &self.mime_db, &self.thumbnails, &self.icons);
        self.watch(path, options);
        *self.current.lock().unwrap() = folder.clone();
        folder
    }

    pub fn back(&self, options: &Options) {
        let path = self.current.lock().unwrap().path.clone();

        if let Some(parent) = path.parent() {
            let folder = self.update(&parent, &options);

            self.proxy.send_event(UserEvent::UpdateFolder {
                folder,
                script_result: None
            });
        }
    }

    pub fn forward(&self, to: &str, options: &Options) {
        let next = self.current.lock().unwrap().path.join(to);

        if next.is_file() {
            if !open(&next) {
                self.update(&next, &options);
                self.proxy.send_event(UserEvent::UpdateFileDeepLook {
                    file: FileMetadata {
                        name: next.file_name()
                            .map(|s| s.to_string_lossy())
                            .unwrap_or_default().to_string(),
                        path: next,
                        graphic: None,
                        openers: vec![]
                    }
                });
            }
        } else {
            let folder = self.update(&next, &options);
            self.proxy.send_event(UserEvent::UpdateFolder {
                folder,
                script_result: None
            });
        }
    }

    pub fn jump(&self, to: &str, options: &Options) -> Option<PathBuf> {
        let home = dirs::home_dir().unwrap();
        let url = Url::parse(&to);
        let scheme = url.clone()
            .map(|s| s.scheme().to_string())
            .unwrap_or_else(|_| String::new());
        let path = match &*scheme {
            "file" => match &to[7..] {
                "~" => home,
                s @ _ if s.starts_with("~/") => {
                    home.join(&s[2..])
                },
                s @ _ if s == "Trash" || s == "trash" => {
                    dirs::data_dir().unwrap().join("Trash").join("files")
                },
                s @ _ => {
                    PathBuf::from(s)
                }
            },
            "trash" => {
                dirs::data_dir().unwrap().join("Trash").join("files")
            },
            _ => {
                return None
            }
        };

        if !path.exists() {
            self.proxy.send_event(UserEvent::NonexistentFolder {
                path: String::from(to)
            });

            None
        } else {
            let mut folder = self.update(&path, &options);
            folder.url = match &*scheme {
                "trash" => url.ok(),
                _ => None
            };

            self.proxy.send_event(UserEvent::UpdateFolder {
                folder,
                script_result: None
            });

            Some(path)
        }
    }

    fn get_folder(
        path: &Path,
        options: &Options,
        mime_db: &SharedMimeInfo,
        thumbnails: &Thumbnails,
        icons: &Icons) -> Folder
    {
        let files = if path.is_dir() {
            let (mut folders, mut files) = fs::read_dir(&path)
                .unwrap()
                .into_iter()
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();

                    if name.starts_with(".") && !options.sort_show_hidden {
                        return None;
                    }

                    Some(entry)
                })
                .fold((vec![], vec![]), |(mut folders, mut files), entry| {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        folders.push(entry);
                    } else {
                        files.push(entry);
                    }

                    (folders, files)
                });

            let files = if options.sort_folders_first {
                folders.sort_by(|a, b| {
                    let a_name = a.file_name().to_string_lossy().into_owned();
                    let b_name = b.file_name().to_string_lossy().into_owned();

                    match options.sort {
                        Sort::AToZ => a_name.cmp(&b_name),
                        Sort::ZToA => b_name.cmp(&a_name),
                        Sort::Date => Ordering::Equal
                    }
                });
                files.sort_by(|a, b| {
                    let a_name = a.file_name().to_string_lossy().into_owned();
                    let b_name = b.file_name().to_string_lossy().into_owned();

                    match options.sort {
                        Sort::AToZ => a_name.cmp(&b_name),
                        Sort::ZToA => b_name.cmp(&a_name),
                        Sort::Date => Ordering::Equal
                    }
                });
                folders.into_iter().chain(files.into_iter()).collect::<Vec<_>>()
            } else {
                let mut joined = folders.into_iter().chain(files.into_iter()).collect::<Vec<_>>();
                joined.sort_by(|a, b| {
                    let a_name = a.file_name().to_string_lossy().into_owned();
                    let b_name = b.file_name().to_string_lossy().into_owned();

                    match options.sort {
                        Sort::AToZ => a_name.cmp(&b_name),
                        Sort::ZToA => b_name.cmp(&a_name),
                        Sort::Date => {
                            let a_modified = a.metadata().ok().map(|m| m.modified().ok()).flatten();
                            let b_modified = b.metadata().ok().map(|m| m.modified().ok()).flatten();

                            match (a_modified, b_modified) {
                                (Some(a), Some(b)) => a.cmp(&b),
                                _ => Ordering::Equal
                            }
                        }
                    }
                });
                joined
            };

            let cache_mtime = icons.get_cache_mtime();

            files.into_iter()
                .map(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let mut icon_url = |entry: DirEntry| {
                        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            Some(get_folder_icon_url(&entry.path()))
                        } else {
                            Some(get_file_icon_url(&entry.path(), mime_db, icons))
                        }
                    };
                    let ext = name.split('.').rev().next();
                    let kind = entry.file_type()
                        .map(|kind| if kind.is_dir() {
                            FolderListingType::Folder
                        } else if kind.is_symlink() {
                            FolderListingType::Link
                        } else {
                            FolderListingType::File
                        })
                        .unwrap_or(FolderListingType::File);

                    let guess = mime_db.guess_mime_type()
                        .file_name(&name)
                        .guess();

                    let mut graphic = match (guess.uncertain(), guess.mime_type()) {
                        (false, mime) if mime.type_() == mime::IMAGE => {
                            let path = entry.path();

                            thumbnails.url_from(&path).or_else(|| {
                                thumbnails.generate(&path);
                                icon_url(entry)
                            })
                        },
                        _ => icon_url(entry),
                    };

                    if let Some(graphic) = &mut graphic {
                        graphic.set_query(Some(&format!("v={}", cache_mtime)));
                    }

                    FolderListing { name, kind, graphic }
                })
                .collect::<Vec<FolderListing>>()
        } else {
            vec![]
        };

        Folder {
            path: path.to_path_buf(),
            url: None,
            files
        }
    }

    fn watch(&self, path: &Path, options: &Options) {
        *self.options.lock().unwrap() = options.clone();

        let current_path = self.current_path();
        let mut debouncer = self.debouncer.lock().unwrap();

        let _ = debouncer.watcher().unwatch(&current_path);
        if path.is_dir() {
            debouncer
                .watcher()
                .watch(path, RecursiveMode::NonRecursive)
                .unwrap();
        }
    }
}

fn get_file_icon_url(path: &Path, mime_db: &SharedMimeInfo, icons: &Icons) -> Url {
    let names = mime_db
        .get_mime_types_from_file_name(path.to_str().unwrap())
        .into_iter()
        .filter_map(|mime| mime_db.lookup_icon_names(&mime).first().map(|s| s.to_string()))
        .collect::<Vec<String>>();

    let names_length = names.len();
    let mut uncached = vec![];

    if !icons.cache_is_stale() {
        for mut icon_name in names {
            // xdg-mime's default icon here is application-octet-stream. We're changing that to
            // text-x-generic because it's usually a bit neutral.
            if names_length == 1 && icon_name == "application-octet-stream" {
                icon_name = "text-x-generic".to_string();
            }

            match icons.get_cached(&icon_name, 256, 1) {
                Some(Some(_)) => { return Url::parse(&format!("icon://{}", &icon_name)).unwrap() }
                Some(None) => {}
                None => { uncached.push(icon_name.clone()); }
            }
        }
    } else {
        uncached = names;
    }

    for icon_name in uncached.iter() {
        if let Ok(_) = icons.find(&icon_name, 256, 1) {
            return Url::parse(&format!("icon://{}", &icon_name)).unwrap()
        }
    }

    Url::parse("icon://text-x-generic").unwrap()
}

fn get_folder_icon_url(path: &Path) -> Url {
    let paths = path.to_str().unwrap_or("").split("/");
    let paths = paths.take(5).collect::<Vec<_>>();

    match (paths.len(), paths.as_slice()) {
        (4, [_, "home", _, "Music"]) => Url::parse("icon://folder-music").unwrap(),
        (4, [_, "home", _, "Pictures"]) => Url::parse("icon://folder-pictures").unwrap(),
        (4, [_, "home", _, "Documents"]) => Url::parse("icon://folder-documents").unwrap(),
        (4, [_, "home", _, "Downloads"]) => Url::parse("icon://folder-download").unwrap(),
        (4, [_, "home", _, "Desktop"]) => Url::parse("icon://user-desktop").unwrap(),
        (4, [_, "home", _, "Dropbox"]) => Url::parse("icon://folder-dropbox").unwrap(),
        (4, [_, "home", _, "Public"]) => Url::parse("icon://folder-publicshare").unwrap(),
        (4, [_, "home", _, "Templates"]) => Url::parse("icon://folder-templates").unwrap(),
        (4, [_, "home", _, "Videos"]) => Url::parse("icon://folder-videos").unwrap(),
        _ => Url::parse(&format!("icon://folder")).unwrap()
    }
}

#[cfg(target_os = "linux")]
pub fn open(path: impl AsRef<OsStr>) -> bool {
    let mut child = Command::new("xdg-open")
        .args(&[ path ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .expect("xdg-open failed");

    std::thread::sleep(Duration::from_millis(100));
    match child.try_wait() {
        Ok(Some(status)) => match status.code() {
            Some(XDG_OPEN_ERROR_APPLICATION_NOT_FOUND) |
            Some(XDG_OPEN_ERROR_ACTION_FAILED) => false,
            _ => true
        },
        Ok(None) => true,
        _ => false
    }
}

#[cfg(target_os = "macos")]
pub fn open(path: impl AsRef<OsStr>) -> bool {
    let mut child = Command::new("xdg-open")
        .args(&[ path ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .expect("open failed");

    std::thread::sleep(Duration::from_millis(100));
    match child.try_wait() {
        Ok(Some(status)) => status.success(),
        Ok(None) => true,
        _ => false
    }
}

const XDG_OPEN_ERROR_APPLICATION_NOT_FOUND: i32 = 3;
const XDG_OPEN_ERROR_ACTION_FAILED: i32 = 4;
