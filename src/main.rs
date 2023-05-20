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

mod constants;
mod ipc;
mod models;
mod icons;
mod thumbnails;
mod prompts;
mod store;

use ipc::*;
use models::{Action,Account,AccountDirect,AccountAerome,ConversationItem,Suggestions,Folder,FolderListing,FileMetadata,FolderListingType,Options,Sort,Settings};
use icons::Icons;
use tokio::{runtime::{Runtime},process::Command};
use tokio::io::{BufReader,AsyncBufReadExt,AsyncWriteExt,AsyncReadExt};
use tokio::task::AbortHandle;
use std::borrow::Cow;
use std::fs;
use std::ffi::OsStr;
use std::path::{PathBuf,Path};
use std::process::Stdio;
use std::sync::{Arc,Mutex};
use std::env::current_dir;
use std::fs::DirEntry;
use std::cmp::Ordering;
use std::time::Duration;
use std::thread;
use wry::{
    application::{
        event::{Event,StartCause,WindowEvent},
        event_loop::{ControlFlow,EventLoop,EventLoopProxy},
        window::{WindowBuilder,Window},
    },
    http::{
        header::CONTENT_TYPE,
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        Response,
        StatusCode
    },
    webview::{
        WebView,
        WebViewBuilder
    }
};
use url::Url;
use serde_json::json;
use thumbnails::{ThumbnailSize,Thumbnails};
use xdg_mime::{SharedMimeInfo, Guess};
use store::Store;
use prompt::{PromptArgs,EvaluateError,EvaluateResult,evaluate};
use ai::{ChatError, ChatError::OpenAIError};

fn main() -> wry::Result<()> {
    env_logger::init();
    constants::install();

    let store = Store::new();
    let icons = Mutex::new(Icons::new());
    let mime_db = SharedMimeInfo::new();
    let event_loop = EventLoop::<UserEvent>::with_user_event();
    let (thumbnails, handle) = Thumbnails::new(event_loop.create_proxy());
    let folder = Arc::new(Mutex::new(
        get_folder(&current_dir().unwrap(), &Options::default(), &mime_db, &thumbnails)));
    let proxy = event_loop.create_proxy();
    let rt = Runtime::new().unwrap();
    let theme = Icons::get_current_theme_name();

    let handler_folder = folder.clone();
    let handler_thumbnails = thumbnails.clone();
    let handler = move |window: &Window, req: String| {
        match serde_json::from_str(req.as_str()).unwrap() {
            Cmd::Dev => {
                if cfg!(debug_assertions) {
                    proxy.send_event(UserEvent::DevTools);
                }
            },
            Cmd::Initialized => {
                let path = std::env::current_dir().unwrap()
                    .join("assets").join("icon").join("icon")
                    .with_extension("png");

                handler_thumbnails.generate(&path);
                window.set_visible(true);
                proxy.send_event(UserEvent::UpdateSettings {
                    settings: store.get_settings()
                });
            },
            Cmd::Back { options } => {
                let mut unlocked = handler_folder.lock().unwrap();

                if let Some(parent) = unlocked.path.parent() {
                    *unlocked = get_folder(&parent, &options, &mime_db, &handler_thumbnails);

                    proxy.send_event(UserEvent::UpdateFolder {
                        folder: (*unlocked).clone(),
                        script_result: None
                    });
                }
            },
            Cmd::Forward { to, options } => {
                let mut unlocked = handler_folder.lock().unwrap();
                let next = unlocked.path.join(to);

                if next.is_file() {
                    if !open(&next) {
                        *unlocked = get_folder(&next, &options, &mime_db, &handler_thumbnails);
                        proxy.send_event(UserEvent::UpdateFileDeepLook {
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
                    *unlocked = get_folder(&next, &options, &mime_db, &handler_thumbnails);
                    proxy.send_event(UserEvent::UpdateFolder {
                        folder: (*unlocked).clone(),
                        script_result: None
                    });
                }
            },
            Cmd::Jump { to, options } => {
                let path = if to.starts_with("~/") {
                    if let Some(home) = dirs::home_dir() {
                        home.join(&to[2..])
                    } else {
                        PathBuf::from(&to)
                    }
                } else {
                    PathBuf::from(&to)
                };

                if !path.exists() {
                    proxy.send_event(UserEvent::NonexistentFolder {
                        path: to
                    });
                } else {
                    let mut unlocked = handler_folder.lock().unwrap();
                    *unlocked = get_folder(&path, &options, &mime_db, &handler_thumbnails);

                    proxy.send_event(UserEvent::UpdateFolder {
                        folder: (*unlocked).clone(),
                        script_result: None
                    });
                }
            },
            Cmd::Window(WindowCmd::Drag) => {
                let _ = window.drag_window();
            },
            Cmd::Window(WindowCmd::Maximize) => {
                window.set_maximized(!window.is_maximized());
            },
            Cmd::Window(WindowCmd::Minimize) => {
                window.set_minimized(true);
            },
            Cmd::Window(WindowCmd::Close) => {
                proxy.send_event(UserEvent::CloseWindow);
            },
            Cmd::Communicate { message } => {
                let unlocked = handler_folder.lock().unwrap();
                let settings = store.get_settings();

                if let Some(account) = settings.account {
                    communicate(&rt, &message, proxy.clone(), &unlocked.clone(), &account);
                }
            },
            Cmd::Evaluate { item, options } if item.code.is_some() => {
                let mut unlocked = handler_folder.lock().unwrap();
                let script = format!("{}\n echo -e {}",
                    item.code.as_ref().unwrap(),
                    r#""\n""#);

                let result = run_script_sync(script, &unlocked.path);
                *unlocked = get_folder(&unlocked.path, &options, &mime_db, &handler_thumbnails);

                match (&result, item.message) {
                    (Ok(_), Some(message)) => {
                        maybe_add_suggestion(
                            &rt, proxy.clone(), unlocked.path.clone(), message, item.code.unwrap());
                    },
                    _ => {}
                }

                proxy.send_event(UserEvent::UpdateFolder {
                    folder: (*unlocked).clone(),
                    script_result: Some(result
                        .map(|r| ConversationItem::new(
                            format!("Command finished with result:\n\n{r}"), None))
                        .unwrap_or_else(|r| ConversationItem::new(
                            format!("Command finished with error:\n\n{r}"), None)))
                });
            },
            Cmd::Options { options } => {
                let mut unlocked = handler_folder.lock().unwrap();
                *unlocked = get_folder(&unlocked.path, &options, &mime_db, &handler_thumbnails);

                proxy.send_event(UserEvent::UpdateFolder {
                    folder: (*unlocked).clone(),
                    script_result: None
                });
            },
            Cmd::Settings { settings } => {
                store.set_account(&settings.account);
                proxy.send_event(UserEvent::UpdateSettings { settings });
            }
            _ => {}
        }
    };

    let window = WindowBuilder::new()
        .with_title("Future")
        .with_decorations(false)
        .with_transparent(true)
        .build(&event_loop)?;

    window.set_visible(false);

    let webview = WebViewBuilder::new(window)?
        .with_html(include_str!("../www/index.html"))?
        .with_background_color((0, 0, 0, 1))
        .with_ipc_handler(handler)
        .with_transparent(true)
        .with_new_window_req_handler(|url| match &*url {
            "https://aerome.net/tos.html" |
            "https://aerome.net/privacy_policy.html" => { open(url); false },
            _ => false
        })
        .with_custom_protocol("icon".into(), move |req| {
            let mut ic = icons.lock().unwrap();
            let icon = req.uri().host().unwrap();
            let size = Url::parse(&format!("{}", req.uri())).unwrap().query_pairs()
                .find(|(name, _)| &*name == "size")
                .map(|(_, val)| val.to_owned().parse::<i32>().unwrap())
                .unwrap_or(256);

            let path = ic.find(&theme, icon, size, 1)
                .or_else(|_| {
                    eprintln!("Couldn't find {icon} with {size} in {}", theme);
                    ic.find(&theme, "error-symbolic", 32, 1)
                })
                .unwrap();

            let content_type = match path.extension().map(|s| s.to_str().unwrap()) {
                Some("png") => "image/png",
                Some("svg") => "image/svg+xml",
                Some("xpm") => panic!("No idea what to do here"),
                _ => unreachable!()
            };

            Response::builder()
                .header(CONTENT_TYPE, content_type)
                .body(std::fs::read(&path).unwrap().into())
                .map_err(Into::into)
        })
        .with_custom_protocol("thumbnail".into(), move |req| {
            Response::builder()
                .header(CONTENT_TYPE, "image/png")
                .body(thumbnails.find(&req.uri(), ThumbnailSize::Large).into())
                .map_err(Into::into)
        })
        .build()?;

    let event_loop_folder = folder.clone();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(UserEvent::DevTools) => {
                open_devtools(&webview);
            },

            Event::UserEvent(UserEvent::UpdateSuggestions { description }) => {
                let stringified = serde_json::to_string(&description).unwrap();
                webview.evaluate_script(&format!("setSuggestions({})", &stringified)).unwrap();
            },

            Event::UserEvent(UserEvent::NonexistentFolder { path }) => {
                let stringified = serde_json::to_string(&json!({ "path": path })).unwrap();
                webview.evaluate_script(&format!("setMissingFolder({})", &stringified)).unwrap();
            },
    
            Event::UserEvent(UserEvent::UpdateThumbnail { thumbnail }) => {
                let stringified = serde_json::to_string(&thumbnail).unwrap();
                webview.evaluate_script(&format!("updateThumbnail({})", &stringified)).unwrap();
            },

            Event::UserEvent(UserEvent::UpdateFileDeepLook { file }) => {
                let stringified = serde_json::to_string(&file).unwrap();
                webview.evaluate_script(&format!("setFileDeepLook({})", &stringified)).unwrap();
            },

            Event::UserEvent(UserEvent::UpdateSettings { settings }) => {
                let stringified = serde_json::to_string(&settings).unwrap();
                webview.evaluate_script(&format!("setSettings({})", &stringified)).unwrap();
            },

            Event::UserEvent(UserEvent::UpdateFolder { folder, script_result }) => {
                let stringified = serde_json::to_string(&folder).unwrap();
                let suggestions = Store::new().get_suggestions(&folder.path);
                let suggestions = serde_json::to_string(&suggestions).unwrap();

                webview.evaluate_script(&format!("setFolder({})", &stringified)).unwrap();

                match script_result {
                    Some(result) if matches!(result.message, Some(_)) => {
                        let item = serde_json::to_string(&result).unwrap();
                        webview.evaluate_script(&format!("addConversationItem({item})")).unwrap();
                    },
                    Some(_) => {
                        webview.evaluate_script("closeActionsBox()").unwrap();
                    },
                    _ => {}
                }

                if suggestions.len() > 0 {
                    webview.evaluate_script(&format!("setSuggestions({suggestions})")).unwrap();
                }
            },

            Event::UserEvent(UserEvent::Ai(response)) => match response {
                AiResponse::Success(success) => {
                    let message = "Sure, I can do that. Please review this script before evaluating it:";
                    let item = ConversationItem::new(message.to_string(), Some(success));
                    let item = serde_json::to_string(&item).unwrap();

                    webview.evaluate_script(&format!("addConversationItem({item})")).unwrap();
                },
                AiResponse::Failure(failure) => {
                    let item = ConversationItem::new(failure, None);
                    let item = serde_json::to_string(&item).unwrap();
                    webview.evaluate_script(&format!("addConversationItem({item})")).unwrap();
                },
            },
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } |
            Event::UserEvent(UserEvent::CloseWindow) => {
                //let _ = webview.take();
                *control_flow = ControlFlow::Exit;
            },
            _ => (),
        }
    });

    Ok(())
}

#[cfg(debug_assertions)]
fn open_devtools(webview: &WebView) {
    webview.open_devtools();
}

#[cfg(not(debug_assertions))]
fn open_devtools(webview: &WebView) {}

#[cfg(target_os = "linux")]
fn open(path: impl AsRef<OsStr>) -> bool {
    Runtime::new()
        .unwrap()
        .block_on(async move {
            let result = Command::new("xdg-open")
                .args(&[ path ])
                .output()
                .await
                .unwrap();

            match result.status.code() {
                Some(XDG_OPEN_ERROR_APPLICATION_NOT_FOUND) |
                Some(XDG_OPEN_ERROR_ACTION_FAILED) => false,
                _ => true
            }
        })
}

#[cfg(target_os = "macos")]
fn open(path: impl AsRef<OsStr>) -> bool {
    Runtime::new().unwrap().block_on(async move {
        Command::new("open")
            .args(&[ &path ])
            .output()
            .await
            .unwrap()
            .status
            .success()
    })
}

const XDG_OPEN_ERROR_APPLICATION_NOT_FOUND: i32 = 3;
const XDG_OPEN_ERROR_ACTION_FAILED: i32 = 4;

fn communicate(
    rt: &Runtime,
    message: &str,
    proxy: EventLoopProxy<UserEvent>,
    folder: &Folder,
    account: &Account)
{
    let dir = folder.path.to_string_lossy();
    let files = folder.files.iter()
        .filter_map(|f| matches!(f.kind, FolderListingType::File).then(|| f.name.to_string()))
        .collect::<Vec<_>>().join(", ");
    let folders = folder.files.iter()
        .filter_map(|f| matches!(f.kind, FolderListingType::Folder).then(|| f.name.to_string()))
        .collect::<Vec<_>>().join(", ");

    let message = format!(r#"Given these files "{files}" and folders "{folders}" in this directory "{dir}". {message}"#);

    let account = account.clone();
    rt.spawn(async move {
        let result = run_prompt("communicate.pr", &message, &account).await;
        let (kind, message) = result.split_once(":")
            .unwrap_or_else(|| ("FAILURE", "I'm sorry I don't understand, can you try again?"));

        proxy.send_event(UserEvent::Ai(match kind {
            "SUCCESS" => AiResponse::Success(message.to_string()),
            _ => AiResponse::Failure(message.to_string()),
        }));
    });
}

fn maybe_add_suggestion(
    rt: &Runtime,
    proxy: EventLoopProxy<UserEvent>,
    path: PathBuf,
    message: String,
    code: String)
{
    rt.spawn(async move {
        let store = Store::new();
        if let Some(account) = store.get_settings().account {
            let description = Some(run_prompt("summary.pr", &message, &account).await);
            store.add_suggestion(&path, &Action {
                code,
                description,
                question: message
            });
            proxy.send_event(UserEvent::UpdateSuggestions {
                description: store.get_suggestions(&path)
            });
        }
    });
}

fn run_script_sync(bash_script: String, current_dir: &Path) -> Result<String, String> {
    Runtime::new()
        .unwrap()
        .block_on(async move {  run_script(bash_script, current_dir).await })
}

async fn run_script(bash_script: String, current_dir: &Path) -> Result<String, String> {
    let mut cmd = Command::new("/bin/bash")
        .args(&[ "-c", &bash_script ])
        .current_dir(current_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .unwrap();

    if cmd.status.success() {
        Ok(String::from_utf8(cmd.stdout).unwrap())
    } else {
        Err(String::from_utf8(cmd.stderr).unwrap())
    }
}

async fn run_prompt(prompt_path: &str, input: &str, account: &Account) -> String {
    let prompts_dir = dirs::data_local_dir()
        .map(|data_dir| data_dir.join(constants::APP_NAME).join("prompts"))
        .expect("Could not find the apps data directory");

    let (api_key, api_proxy) = match account {
        Account::Direct(key) => (Some(key.0.clone()), None),
        Account::Aerome(AccountAerome { key, .. }) => (
            Some(key.clone()),
            Some(constants::BACKEND_URL.to_owned())
        ),
    };

    let args = PromptArgs {
        path: prompts_dir.join(prompt_path),
        quiet: false,
        api_key,
        api_proxy,
        append: Some(input.to_owned()),
        test: None,
        watch: None
    };

    let out = PromptOut(Vec::new());
    let result = evaluate(args, out).await;

    match result {
        Ok(out) => String::from_utf8(out.0).unwrap(),
        Err(e) => {
            let message = match account {
                Account::Direct(_) => match e {
                    EvaluateError::ChatError(ChatError::NetworkError(e)) => {
                        format!("{}", e.without_url())
                    },
                    EvaluateError::ChatError(ChatError::OpenAIError(e)) if e.status == 401 => {
                        String::from("Invalid API key")
                    },
                    EvaluateError::ChatError(ChatError::OpenAIError(e)) if e.status == 429 => {
                        String::from(
                            "You've exceeded your quota or Open AI's servers are overloaded")
                    },
                    EvaluateError::ChatError(ChatError::OpenAIError(e)) if e.status == 500 => {
                        String::from("Open AI's server experienced an internal error")
                    },
                    _ => {
                        eprintln!("{e:#?}");
                        String::from("The AI assistant failed with an unknown error, sorry!")
                    }
                },
                Account::Aerome(_) => match e {
                    EvaluateError::ChatError(ChatError::NetworkError(e)) => {
                        format!("{}", e.without_url())
                    },
                    EvaluateError::ChatError(ChatError::OpenAIError(e)) if e.status == 401 => {
                        String::from("Invalid API key")
                    },
                    EvaluateError::ChatError(ChatError::OpenAIError(e)) if e.status == 429 => {
                        String::from("\
                            You've exceeded your monthly quota. You can buy more credits in the \
                            account page.")
                    },
                    _ => {
                        eprintln!("{e:#?}");
                        String::from("The AI assistant failed with an unknown error, sorry!")
                    }
                }
            };

            format!("FAILURE: {message}")
        }
    }

}

#[derive(Debug)]
struct PromptOut(Vec<u8>);

impl std::io::Write for PromptOut {
    fn flush(&mut self) -> Result<(), std::io::Error> {
        std::io::Write::flush(&mut self.0)
    }
    fn write(&mut self, d: &[u8]) -> Result<usize, std::io::Error> {
        std::io::Write::write(&mut self.0, d)
    }
}

impl Clone for PromptOut {
    fn clone(&self) -> Self { PromptOut(self.0.to_owned()) }
}

fn get_folder(
    path: &Path,
    options: &Options,
    mime_db: &SharedMimeInfo,
    thumbnails: &Thumbnails) -> Folder
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
                    },
                }
            });
            joined
        };

        files.into_iter()
            .map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                let icon_url = |entry: DirEntry| {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        Some(get_folder_icon_url(&entry.path()))
                    } else {
                        Some(get_file_icon_url(&entry.path()))
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

                let graphic = match (guess.uncertain(), guess.mime_type()) {
                    (false, mime) if mime.type_() == mime::IMAGE => {
                        let path = entry.path();

                        thumbnails.url_from(&path).or_else(|| {
                            thumbnails.generate(&path);
                            icon_url(entry)
                        })
                    },
                    _ => icon_url(entry),
                };

                FolderListing {
                    name,
                    kind,
                    graphic
                }
            })
            .collect::<Vec<FolderListing>>()
    } else {
        vec![]
    };

    Folder {
        path: path.to_path_buf(),
        files
    }
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
        _ => Url::parse("icon://folder").unwrap()
    }
}

fn get_file_icon_url(path: &Path) -> Url {
    // TODO
    Url::parse("icon://text-x-generic").unwrap()
    /*
     * figured this one out on linux, use the cli program: xdg-mime query filetype {filename}
     * it'll return the mimetype thats used as an icon we can look up, so just:

     * icon://$(xdg-mime query filetype {filename})

     * will work. The man page has links to the spec, which links to c libraries for mime
     * lookups. For Mac OS I probably need to import that library and install the entires
     * /usr/share/mime folder if i want it to work. I could also take a look at how Mac OS
     * handles mimetypes, but given I've already leaned into the Yaru theme, this is the
     * faster path
     */
}
