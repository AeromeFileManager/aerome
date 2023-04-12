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

mod ipc;
mod models;
mod icons;

use ipc::*;
use models::{Suggestions,Folder,File,Options,Sort};
use icons::Icons;
use tokio::{runtime::{Runtime},process::Command};
use tokio::io::{BufReader,AsyncBufReadExt,AsyncWriteExt,AsyncReadExt};
use tokio::task::AbortHandle;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc,Mutex};
use std::env::current_dir;
use std::fs;
use std::cmp::Ordering;
use std::time::Duration;
use wry::{
    application::{
        event::{Event,StartCause,WindowEvent},
        event_loop::{ControlFlow,EventLoop,EventLoopProxy},
        window::{WindowBuilder,Window},
    },
    http::{header::CONTENT_TYPE,Response},
    webview::WebViewBuilder
};
use url::Url;
use std::path::Path;
use std::thread;
use serde_json::json;

fn main() -> wry::Result<()> {
    let folder = Arc::new(Mutex::new(get_folder(&current_dir().unwrap(), &Options::default())));
    let event_loop = EventLoop::<UserEvent>::with_user_event();
    let proxy = event_loop.create_proxy();
    let rt = Runtime::new().unwrap();
    let theme = get_icon_theme(&rt);
    let investigation: Mutex<AbortHandle> = Mutex::new(
        investigate(&rt, (*folder.lock().unwrap()).clone(), proxy.clone()));

    let handler_folder = folder.clone();
    let handler = move |window: &Window, req: String| {
        match serde_json::from_str(req.as_str()).unwrap() {
            Cmd::Initialized => {
                window.set_visible(true);
            },
            Cmd::Back { options } => {
                let mut unlocked = handler_folder.lock().unwrap();

                if let Some(parent) = unlocked.path.parent() {
                    *unlocked = get_folder(&parent, &options);

                    proxy.send_event(UserEvent::UpdateFolder {
                        folder: (*unlocked).clone()
                    });

                    /*
                    let mut ongoing = investigation.lock().unwrap();
                    ongoing.abort();

                    *ongoing = investigate(&rt, unlocked.clone(), proxy.clone());
                    */
                }
            },
            Cmd::Forward { to, options } => {
                let mut unlocked = handler_folder.lock().unwrap();
                *unlocked = get_folder(&unlocked.path.join(to), &options);

                proxy.send_event(UserEvent::UpdateFolder {
                    folder: (*unlocked).clone()
                });

                /*
                let mut ongoing = investigation.lock().unwrap();
                ongoing.abort();

                *ongoing = investigate(&rt, unlocked.clone(), proxy.clone());
                */
            },
            Cmd::Jump { to, options } => {
                let path = Path::new(&to);

                if !path.exists() {
                    proxy.send_event(UserEvent::NonexistentFolder {
                        path: to
                    });
                } else {
                    let mut unlocked = handler_folder.lock().unwrap();
                    *unlocked = get_folder(&path, &options);

                    proxy.send_event(UserEvent::UpdateFolder {
                        folder: (*unlocked).clone()
                    });
                }
            },
            Cmd::Window(WindowCmd::Drag) => {
                let _ = window.drag_window();
            },
            Cmd::Communicate { message } => {
                communicate(&rt, &message, proxy.clone());
            },
            Cmd::Options { options } => {
                let mut unlocked = handler_folder.lock().unwrap();
                *unlocked = get_folder(&unlocked.path, &options);

                proxy.send_event(UserEvent::UpdateFolder {
                    folder: (*unlocked).clone()
                });
            },
            _ => {}
        }
    };

    let window = WindowBuilder::new()
        .with_title("Future")
        .with_decorations(false)
        .with_transparent(true)
        .build(&event_loop)?;

    window.set_visible(false);

    let icons = Mutex::new(Icons::default());

    let webview = WebViewBuilder::new(window)?
        .with_html(include_str!("../www/index.html"))?
        .with_background_color((0, 0, 0, 1))
        .with_ipc_handler(handler)
        .with_transparent(true)
        .with_custom_protocol("icon".into(), move |req| {
            let mut ic = icons.lock().unwrap();
            let icon = req.uri().host().unwrap();
            let size = Url::parse(&format!("{}", req.uri())).unwrap().query_pairs()
                .find(|(name, _)| &*name == "size")
                .map(|(_, val)| val.to_owned().parse::<i32>().unwrap())
                .unwrap_or(256);

            let path = ic.find(&theme, icon, size, 1).unwrap();

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
        .with_custom_protocol("thumbnail".into(), |req| {
            // https://specifications.freedesktop.org/thumbnail-spec/thumbnail-spec-latest.html

            let hash = req.uri().host().unwrap();
            let path = format!("/home/linuser/.cache/thumbnails/large/{}.png", hash);

            Response::builder()
                .header(CONTENT_TYPE, "image/png")
                .body(std::fs::read(&path)
                    .or_else(|_| {
                        let path = format!("/home/linuser/.cache/thumbnails/medium/{}.png", hash);
                        std::fs::read(&path)
                    })
                    .or_else(|_| std::fs::read("./assets/e08239b208f592aa0081561aecb42a3b.png"))
                    .unwrap()
                    .into())
                .map_err(Into::into)
        })
        .build()?;

    let event_loop_folder = folder.clone();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(UserEvent::UpdateSuggestions { description }) => {
                let stringified = serde_json::to_string(&description).unwrap();
                webview.evaluate_script(&format!("setSuggestions({})", &stringified)).unwrap();
            },

            Event::UserEvent(UserEvent::NonexistentFolder { path }) => {
                let stringified = serde_json::to_string(&json!({ "path": path })).unwrap();
                webview.evaluate_script(&format!("setMissingFolder({})", &stringified)).unwrap();
            },
    
            Event::UserEvent(UserEvent::UpdateFolder { folder }) => {
                let stringified = serde_json::to_string(&folder).unwrap();
                webview.evaluate_script(&format!("setFolder({})", &stringified)).unwrap();
            },

            Event::UserEvent(UserEvent::Ai(response)) => match response {
                AiResponse::Success(success) => {
                    let message = "Sure, I can do that. Please review this script before evaluating it:";
                    let code = success.replace('`', "\\`");
                    let script = &format!("addConversationItem('ai', `{message}`, `{code}`)");

                    webview.evaluate_script(script).unwrap();
                },
                AiResponse::Failure(failure) => {
                    let message = failure.replace('`', "\\`");
                    webview.evaluate_script(&format!("addConversationItem('ai', `{}`)", message))
                        .unwrap();
                },
            },

            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } =>
                *control_flow = ControlFlow::Exit,

            _ => (),
        }
    });

    Ok(())
}

fn communicate(rt: &Runtime, message: &str, proxy: EventLoopProxy<UserEvent>) {
    let message = message.to_string();

    rt.spawn(async move {
        let result = run_prompt("prompts/communicate.pr", message.as_bytes()).await;
        let (kind, message) = result.split_once(":")
            .unwrap_or_else(|| ("FAILURE", "I'm sorry I don't understand, can you try again?"));

        proxy.send_event(UserEvent::Ai(match kind {
            "SUCCESS" => AiResponse::Success(message.to_string()),
            _ => AiResponse::Failure(message.to_string()),
        }));
    });
}

async fn run_prompt(prompt_path: &str, input: &[u8]) -> String {
    let path = std::env::current_dir().unwrap().join("./bin/prompt");
    let mut cmd = Command::new(path)
        .arg(prompt_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = cmd.stdin.take().unwrap();
    stdin.write_all(input).await.unwrap();
    stdin.write_all(b"\n").await.unwrap();

    let mut stdout = cmd.stdout.take().unwrap();
    let mut result = String::new();
    stdout.read_to_string(&mut result).await.unwrap();

    result
}

fn investigate(rt: &Runtime, folder: Folder, proxy: EventLoopProxy<UserEvent>) -> AbortHandle {
    rt.spawn(async move {
        let path = std::env::current_dir().unwrap().join("./bin/prompt");
        let mut cmd = Command::new(path)
            .arg("prompts/describe.pr")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let mut stdin = cmd.stdin.take().unwrap();
        let files = folder.files.iter()
            .map(|f| format!("{}", f.name))
            .collect::<Vec<_>>()
            .join(", ");

        stdin.write_all(files.as_bytes()).await.unwrap();
        stdin.write_all(b"\n").await.unwrap();

        let mut stdout = cmd.stdout.take().unwrap(); 
        let mut result = String::new();
        stdout.read_to_string(&mut result).await.unwrap();

        let mut lines = result.lines();
        let purpose = lines.next().unwrap().replace("\"", "\\\"");
        let actions = lines.next().unwrap();

        /*
        proxy.send_event(UserEvent::UpdateSuggestions {
            description: serde_json::from_str(&format!(r#"{{
                "purpose": "{purpose}",
                "actions": {actions}
            }}"#)).unwrap()
        });
        */
    }).abort_handle()
}

fn get_folder(path: &Path, options: &Options) -> Folder {
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

                let ext = name.split('.').rev().next();
                let graphic = if ext == Some("png") {
                    let hash = generate_thumbnail_hash(entry.path().to_str().unwrap());
                    Some(Url::parse(&format!("thumbnail://{hash}")).unwrap())
                } else {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        Some(get_folder_icon_url(&entry.path()))
                    } else {
                        Some(get_file_icon_url(&entry.path()))
                    }
                };

                File {
                    name,
                    graphic
                }
            })
            .collect::<Vec<File>>()
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
    Url::parse("icon://text-x-plain").unwrap()
}

fn generate_thumbnail_hash(s: &str) -> String {
    format!("{:x}", md5::compute(&Url::parse(&format!("file://{s}")).unwrap().to_string()))
}

fn get_icon_theme(rt: &Runtime) -> String {
    rt.block_on(async {
        let mut cmd = Command::new("gsettings")
            .args(&["get", "org.gnome.desktop.interface", "icon-theme"])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let mut stdout = cmd.stdout.take().unwrap();
        let mut result = String::new();
        stdout.read_to_string(&mut result).await.unwrap();
        result.replace("'", "").trim().to_string()
    })
}
