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

use super::constants;

use rayon::prelude::*;
use serde::{Serialize,Deserialize};
use std::sync::{Arc,Mutex};
use std::path::{Path,PathBuf};
use std::fs::{self,File};
use std::env;
use std::iter;
use std::collections::HashMap;
use std::process::{Command,Stdio};
use std::io::Read;
use std::time::{Duration,Instant,SystemTime,UNIX_EPOCH};

// Relevant standards
// https://specifications.freedesktop.org/icon-theme-spec/icon-theme-spec-latest.html
// https://specifications.freedesktop.org/icon-naming-spec/icon-naming-spec-latest.html

type ThemeName = String;
type ThemePath = String;
type FindResult = Result<PathBuf, IconLookupFailure>;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Icons {
    cache: Arc<Mutex<HashMap<Lookup, Option<PathBuf>>>>,
    cache_mtime: Arc<Mutex<u64>>,
    cache_mtime_last_check: Arc<Mutex<u64>>,
    cache_is_stale: Arc<Mutex<bool>>,
    current_theme_name: Arc<Mutex<String>>,
    themes: Arc<Mutex<HashMap<ThemeName, Theme>>>
}

impl Icons {
    pub fn new_from_cbor() -> Self {
        let cache_dir = dirs::cache_dir().unwrap().join(constants::APP_NAME);
        let cache_file = File::open(cache_dir.join("icons.cache")).ok();

        cache_file
            .and_then(|cache_file| {
                serde_cbor::from_reader(cache_file).ok()
            })
            .unwrap_or_else(|| {
                Icons::new()
            })
    }

    pub fn new() -> Self {
        let icons = Icons {
            cache: Arc::new(Mutex::new(HashMap::new())),
            themes: Arc::new(Mutex::new(Icons::create_default_themes())),
            cache_mtime: Arc::new(Mutex::new(0)),
            cache_mtime_last_check: Arc::new(Mutex::new(0)),
            cache_is_stale: Arc::new(Mutex::new(false)),
            current_theme_name: Arc::new(Mutex::new(Icons::get_current_theme_name()))
        };
        icons.flush_cache();
        icons
    }

    pub fn get_cached(&self, icon: &str, size: i32, scale: i32) -> Option<Option<PathBuf>> {
        let theme = self.current_theme_name.lock().unwrap();
        let lookup = Lookup::new(&*theme, icon, size, scale);
        let cache = self.cache.lock().unwrap();

        cache.get(&lookup).cloned()
    }

    pub fn get_cache_mtime(&self) -> u64 {
        *self.cache_mtime.lock().unwrap()
    }

    pub fn find(&self, icon: &str, size: i32, scale: i32) -> FindResult {
        if self.cache_is_stale() {
            self.flush_cache();
        }

        let mut cache = self.cache.lock().unwrap();
        let mut themes = self.themes.lock().unwrap();
        let theme = self.current_theme_name.lock().unwrap();
        let lookup = Lookup::new(&theme, icon, size, scale);

        if let Some(cached) = cache.get(&lookup) {
            return match cached {
                Some(path) => {
                    Ok(path.to_owned())
                },
                None => Err(IconLookupFailure::IconResolutionFailed(lookup))
            }
        }

        let theme = if let Some(theme) = themes.get(&theme.clone()) {
            theme.clone()
        } else {
            let mut theme = Theme::load(&theme)
                .ok_or(IconLookupFailure::ThemeMissing(theme.to_owned()))?;

            theme.inherits.push(themes.get(constants::FILE_MANAGER_THEME_NAME).unwrap().clone());
            themes.insert(theme.name.clone(), theme.clone());
            theme
        };

        let fallback = if let Some(theme) = themes.get("hicolor") {
            theme.clone()
        } else {
            let theme = Theme::load("hicolor")
                .ok_or(IconLookupFailure::ThemeMissing("hicolor".to_owned()))?;

            themes.insert("hicolor".to_owned(), theme.clone());
            theme
        };

        if let Some(icon) = find_icon(icon, size, scale, &theme, &fallback) {
            cache.insert(lookup, Some(icon.clone()));
            Ok(icon)
        } else {
            cache.insert(lookup.clone(), None);
            Err(IconLookupFailure::IconResolutionFailed(lookup))
        }
    }

    fn flush_cache(&self) {
        *self.cache_is_stale.lock().unwrap() = false;
        *self.cache.lock().unwrap() = HashMap::new();
        *self.themes.lock().unwrap() = Icons::create_default_themes();
        self.cache_common_mimetypes();
    }

    pub fn cache_is_stale(&self) -> bool {
        let mut last_check = self.cache_mtime_last_check.lock().unwrap();
        let duration_since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let seconds_since_epoch = duration_since_epoch.as_secs() as u64;

        if seconds_since_epoch - *last_check < 5 {
            return *self.cache_is_stale.lock().unwrap();
        }

        let mut last_theme = self.current_theme_name.lock().unwrap();
        let mut current_theme = Icons::get_current_theme_name();
        let mut cache_mtime = self.cache_mtime.lock().unwrap();
        let mut cache_is_stale = self.cache_is_stale.lock().unwrap();

        if current_theme != *last_theme {
            *last_theme = current_theme;
            *last_check = seconds_since_epoch;
            *cache_mtime = seconds_since_epoch;
            *cache_is_stale = true;
            return true;
        }

        *last_check = seconds_since_epoch;
        false
    }

    pub fn cache_common_mimetypes(&self) {
        let this = self.clone();

        std::thread::spawn(move || {
            for mimetype in constants::COMMON_MIME_TYPES.iter() {
                this.find(mimetype, 256, 1);
            }
        });
    }

    #[cfg(target_os = "linux")]
    pub fn get_current_theme_name() -> String {
        let mut cmd = Command::new("gsettings")
            .args(&["get", "org.gnome.desktop.interface", "icon-theme"])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let mut stdout = cmd.stdout.take().unwrap();
        let mut result = String::new();
        stdout.read_to_string(&mut result).unwrap();
        result.replace("'", "").trim().to_string()
    }

    #[cfg(not(target_os = "linux"))]
    pub fn get_current_theme_name() -> String {
        String::from(constants::FILE_MANAGER_THEME_NAME)
    }

    fn create_default_themes() -> HashMap<ThemeName, Theme> {
        let mut themes = HashMap::new();

        themes.insert(constants::FILE_MANAGER_THEME_NAME.into(), Theme {
            name: constants::FILE_MANAGER_THEME_NAME.into(),
            directories: vec![
                "places".into(),
                "mimetypes".into(),
                "scalable".into(),
            ],
            inherits: vec![],
            context: vec![
                (String::from("places"), ThemeContext {
                    context: "".into(),
                    size: 256,
                    r#type: ThemeType::Fixed
                }),
                (String::from("mimetypes"), ThemeContext {
                    context: "".into(),
                    size: 256,
                    r#type: ThemeType::Fixed
                }),
                (String::from("scalable"), ThemeContext {
                    context: "".into(),
                    size: 32,
                    r#type: ThemeType::Scalable
                })
            ].into_iter().collect()
        });

        if cfg!(not(target_os = "linux")) {
            // All conformant freedesktop icon implementations have to have a hicolor theme, but its
            // not going to be there on Mac OS, so we add a dummy implementation here
            themes.insert("hicolor".into(), Theme {
                name: "hicolor".into(),
                ..Theme::default()
            });
        }

        themes
    }
}

impl Drop for Icons {
    fn drop(&mut self) {
        let cache_dir = dirs::cache_dir().unwrap().join(constants::APP_NAME);
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir).unwrap();
        }

        let cache_file = File::create(cache_dir.join("icons.cache")).unwrap();
        serde_cbor::to_writer(cache_file, &self).unwrap();
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Theme {
    name: String,
    directories: Vec<String>,
    inherits: Vec<Theme>,
    context: HashMap<ThemePath, ThemeContext>
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct ThemeContext {
    context: String,
    size: i32,
    r#type: ThemeType
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
enum ThemeType {
    #[default]
    Fixed,
    Scalable,
    Threshold
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Lookup {
    icon: String,
    size: i32,
    scale: i32,
    theme: String
}

impl Lookup {
    fn new(theme: &str, icon: &str, size: i32, scale: i32) -> Self {
        Lookup { theme: theme.into(), icon: icon.into(), size, scale }
    }
}

fn find_icon(icon: &str, size: i32, scale: i32, theme: &Theme, fallback: &Theme) -> Option<PathBuf> {
    match find_icon_helper(icon, size, scale, theme) {
        Some(filename) if filename.exists() => {
            return Some(filename)
        },
        _ => {}
    }

    match find_icon_helper(icon, size, scale, fallback) {
        Some(fallback) if fallback.exists() => {
            return Some(fallback);
        },
        _ => {}
    }

    lookup_fallback_icon(icon)
}

fn find_icon_helper(icon: &str, size: i32, scale: i32, theme: &Theme) -> Option<PathBuf> {
    let subdir_list = &theme.directories;
    let basename_list = get_basename_list();
    let mut closest_filename = Mutex::new(None);
    let mut minimal_size = Mutex::new(i32::MAX);

    let mut files = vec![];
    for subdir in subdir_list {
        for directory in &basename_list {
            for extension in &["png", "svg", "xpm"] {
                files.push((subdir, directory, extension));
            }
        }
    }

    let found = files.par_iter().position_first(|(subdir, directory, extension)| {
        let subdir_matches_size = directory_matches_size(&subdir, size, scale, theme);
        let filename = Path::new(directory)
            .join(&theme.name)
            .join(&subdir)
            .join(icon)
            .with_extension(extension);

        if filename.exists() {
            if subdir_matches_size {
                return true;
            }

            if let Some(distance) = directory_size_distance(&subdir, size, scale, theme) {
                let ms = *minimal_size.lock().unwrap();

                if distance < ms {
                    *closest_filename.lock().unwrap() = Some(filename);
                    *minimal_size.lock().unwrap() = distance;
                }
            }
        }

        false
    });

    if let Some((subdir, directory, extension)) = found.map(|i| files[i]) {
        return Some(Path::new(directory)
            .join(&theme.name).join(&subdir).join(icon).with_extension(extension));
    }

    let filename = closest_filename.into_inner().unwrap();
    if let Some(filename) = filename {
        return Some(filename);
    }

    for parent in theme.inherits.iter() {
        if let Some(path) = find_icon_helper(icon, size, scale, parent) {
            return Some(path);
        }
    }

    None
}

fn directory_matches_size(subdir: &str, size: i32, scale: i32, theme: &Theme) -> bool {
    if let Some(context) = theme.context.get(subdir) {
        match context.r#type {
            ThemeType::Fixed => size == context.size && scale == 1,
            ThemeType::Scalable => {
                let min_size = context.size / 2;
                let max_size = context.size * 2;
                min_size <= size * scale && size * scale <= max_size
            },
            ThemeType::Threshold => {
                let threshold = context.size / 4;
                context.size - threshold <= size * scale && size * scale <= context.size + threshold
            },
            _ => false
        }
    } else {
        false
    }
}

fn directory_size_distance(subdir: &str, size: i32, scale: i32, theme: &Theme) -> Option<i32> {
    let context = theme.context.get(subdir)?;

    match context.r#type {
        ThemeType::Fixed => Some(i32::abs(context.size * scale - size)),
        ThemeType::Scalable => {
            let min_size = context.size / 2;
            let max_size = context.size * 2;
            if size * scale < min_size {
                Some(min_size * scale - size * scale)
            } else if size * scale > max_size {
                Some(size * scale - max_size * scale)
            } else {
                None
            }
        },
        ThemeType::Threshold => {
            let threshold = context.size / 4;
            if size * scale < (context.size - threshold) {
                Some((context.size - threshold) * scale - size * scale)
            } else if size * scale > (context.size + threshold) {
                Some(size * scale - (context.size + threshold) * scale)
            } else {
                None
            }
        },
        _ => None
    }
}

fn lookup_fallback_icon(icon: &str) -> Option<PathBuf> {
    let basename_list = get_basename_list();
    for directory in &basename_list {
        for extension in &["png", "svg", "xpm"] {
            let filename = Path::new(directory).join(icon).with_extension(extension);
            if filename.exists() {
                return Some(filename);
            }
        }
    }
    None
}

fn get_basename_list() -> Vec<PathBuf> {
    let home_icons = env::var("HOME")
        .map(|home| {
            let mut path = PathBuf::from(home);
            path.push(".icons");
            path
        })
        .ok();

    let xdg_data_dirs = env::var("XDG_DATA_DIRS")
        .map(|dirs| dirs.split(':')
            .map(|dir| Some({
                let mut path = PathBuf::from(dir);
                path.push("icons");
                path
            }))
            .collect()
        )
        .unwrap_or_else(|_| Vec::new());
    
    iter::once(home_icons)
        .chain(xdg_data_dirs.into_iter())
        .chain(iter::once(Some(PathBuf::from("/usr/share/pixmaps"))))
        .chain(iter::once(dirs::data_local_dir()
            .map(|data_dir| data_dir.join(constants::APP_NAME))
        ))
        .filter_map(|dir| dir)
        .collect()
}

#[derive(Debug)]
pub enum IconLookupFailure {
    ThemeMissing(ThemeName),
    IconResolutionFailed(Lookup)
}

impl Theme {
    fn load(name: &str) -> Option<Self> {
        let theme_ini = get_basename_list().into_iter().find_map(|mut base| {
            base.push(name);
            base.push("index.theme");

            if base.exists() {
                return Some(fs::read_to_string(base).ok()?)
            }

            None
        })?;

        let items: Vec<(String, Vec<String>)> = theme_ini.lines().fold(vec![], |mut items, line| {
            if line.starts_with("[") {
                items.push((line[1..line.len() - 1].to_string(), vec![]));
            } else {
                if let Some(mut item) = items.last_mut() {
                    item.1.push(line.to_owned());
                }
            }
            items
        });

        let mut theme = Theme::default();
        theme.name = name.to_owned();

        for (entry, lines) in items {
            match &*entry {
                "Icon Theme" => {
                    for line in lines {
                        if line.starts_with("Directories=") {
                            theme.directories = line
                                .replacen("Directories=", "", 1)
                                .split(",")
                                .filter(|s| s.len() > 0)
                                .map(|s| s.to_owned())
                                .collect();
                        }

                        if line.starts_with("Inherits=") {
                            let inherits: Vec<String> = line
                                .replacen("Inherits=", "", 1)
                                .split(",")
                                .filter(|s| s.len() > 0)
                                .map(|s| s.to_owned())
                                .collect();

                            theme.inherits = inherits.into_iter()
                                .map(|name| Theme::load(&name).expect(
                                    &format!("Couldn't find theme '{}'", name)))
                                .collect::<Vec<Theme>>();
                        }
                    }
                },
                _ => {
                    let mut context = ThemeContext::default();

                    for line in lines {
                        if line.starts_with("Size=") {
                            context.size = line.replacen("Size=", "", 1).parse::<i32>().unwrap();
                        }

                        if line.starts_with("Type=") {
                            match line.trim() {
                                "Type=Fixed" => context.r#type = ThemeType::Fixed,
                                "Type=Scalable" => context.r#type = ThemeType::Scalable,
                                "Type=Threshold" => context.r#type = ThemeType::Threshold,
                                _ => {}
                            }
                        }

                        if line.starts_with("Context") {
                            context.context = line.replacen("Context=", "", 1);
                        }
                    }

                    theme.context.insert(entry, context);
                }
            }
        }
        Some(theme)
    }
}
