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

mod models;

use serde::{Deserialize,Serialize};
use tokio::{runtime::{Runtime},process::Command};
use tokio::io::{BufReader,AsyncBufReadExt,AsyncWriteExt,AsyncReadExt};
use tokio::task::AbortHandle;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc,Mutex};
use std::env::current_dir;
use std::fs;
use wry::{
    application::{
        event::{Event,StartCause,WindowEvent},
        event_loop::{ControlFlow,EventLoop,EventLoopProxy},
        window::{WindowBuilder,Window},
    },
    http::{header::CONTENT_TYPE,Response},
    webview::WebViewBuilder
};
use models::{Description,Folder,File};
use wry::webview::Url;

fn main() -> wry::Result<()> {
    let folder = Arc::new(Mutex::new(get_folder(current_dir().unwrap())));
    let event_loop = EventLoop::<UserEvent>::with_user_event();
    let proxy = event_loop.create_proxy();
    let rt = Runtime::new().unwrap();
    let investigation: Mutex<AbortHandle> = Mutex::new(
        investigate(&rt, (*folder.lock().unwrap()).clone(), proxy.clone()));

    let handler_folder = folder.clone();
    let handler = move |window: &Window, req: String| {
        match serde_json::from_str(req.as_str()).unwrap() {
            Cmd::Back => {
                let mut unlocked = handler_folder.lock().unwrap();

                if let Some(parent) = unlocked.path.parent() {
                    *unlocked = get_folder(parent.to_owned());

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
            Cmd::Forward { to } => {
                let mut unlocked = handler_folder.lock().unwrap();
                *unlocked = get_folder(unlocked.path.join(to));

                proxy.send_event(UserEvent::UpdateFolder {
                    folder: (*unlocked).clone()
                });

                /*
                let mut ongoing = investigation.lock().unwrap();
                ongoing.abort();

                *ongoing = investigate(&rt, unlocked.clone(), proxy.clone());
                */
            },
            Cmd::Window(WindowCmd::Drag) => {
                let _ = window.drag_window();
            },
            _ => {}
        }
    };

    let window = WindowBuilder::new()
        .with_title("Hello World")
        .with_decorations(false)
        .build(&event_loop)?;

    let webview = WebViewBuilder::new(window)?
        .with_html(include_str!("../www/index.html"))?
        .with_ipc_handler(handler)
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
            Event::NewEvents(StartCause::Init) => {
                let folder = event_loop_folder.lock().unwrap();
                let stringified = serde_json::to_string(&*folder).unwrap();
                webview.evaluate_script(&format!("setFolder({})", &stringified)).unwrap();
            },

            Event::UserEvent(UserEvent::UpdateDescription { description }) => {
                let stringified = serde_json::to_string(&description).unwrap();
                webview.evaluate_script(&format!("setDescription({})", &stringified)).unwrap();
            },
    
            Event::UserEvent(UserEvent::UpdateFolder { folder }) => {
                let stringified = serde_json::to_string(&folder).unwrap();
                webview.evaluate_script(&format!("setFolder({})", &stringified)).unwrap();
            },

            Event::UserEvent(UserEvent::ExecEval()) => {
                webview.evaluate_script("alert('works')").unwrap();
            },

            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } =>
                *control_flow = ControlFlow::Exit,

            _ => (),
        }
    });
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

        proxy.send_event(UserEvent::UpdateDescription {
            description: serde_json::from_str(&format!(r#"{{
                "purpose": "{purpose}",
                "actions": {actions}
            }}"#)).unwrap()
        });
    }).abort_handle()
}

fn get_folder(path: PathBuf) -> Folder {
    let files = if path.is_dir() {
        fs::read_dir(&path)
            .unwrap()
            .into_iter()
            .filter_map(|entry|
                entry
                    .map(|entry| {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        let ext = name.split('.').rev().next();
                        let thumbnail = if ext == Some("png") {
                            /*
                                .parse::<Uri>()
                                .unwrap();
                            let uri = format!("{}", url);
                            */

                            Some(generate_thumbnail_hash(entry.path().to_str().unwrap()))
                        } else {
                            None
                        };

                        File {
                            name,
                            thumbnail
                        }
                    })
                    .ok()
            )
            .collect::<Vec<File>>()
    } else {
        vec![]
    };

    println!("{:?}", files);

    Folder {
        path,
        files
    }
}

fn generate_thumbnail_hash(s: &str) -> String {
    println!("{}", s);
    format!("{:x}", md5::compute(&Url::parse(&format!("file://{s}")).unwrap().to_string()))
}

#[derive(Debug, Serialize, Deserialize)]
struct Task {
    name: String,
    done: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Cmd {
    Init,
    Back,
    Forward {
        to: String
    },
    Communicate {
        message: String
    },
    Window(WindowCmd)
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "window", rename_all = "camelCase")]
pub enum WindowCmd {
    Close,
    Drag,
    Maximize,
    Minimize
}

enum UserEvent {
    ExecEval(),
    UpdateFolder {
        folder: Folder
    },
    UpdateDescription {
        description: Description
    }
}
