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
use models::{Suggestions,Folder,File};
use ipc::*;
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
            Cmd::Communicate { message } => {
                communicate(&rt, &message, proxy.clone());
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

            Event::UserEvent(UserEvent::UpdateSuggestions { description }) => {
                let stringified = serde_json::to_string(&description).unwrap();
                webview.evaluate_script(&format!("setSuggestions({})", &stringified)).unwrap();
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

    Folder {
        path,
        files
    }
}

fn generate_thumbnail_hash(s: &str) -> String {
    format!("{:x}", md5::compute(&Url::parse(&format!("file://{s}")).unwrap().to_string()))
}
