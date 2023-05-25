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

use std::sync::mpsc::{self,Receiver,Sender};
use std::sync::{Arc,Mutex};
use std::thread::{self,JoinHandle};
use wry::application::event_loop::{EventLoopProxy};
use fs_extra::{move_items_with_progress, copy_items_with_progress, dir::{CopyOptions,TransitProcessResult}};
use crate::{FileTransferCmd,FileTransferCmdStart,FileTransferCmdResponse,FileTransferProgress,UserEvent};
use std::collections::VecDeque;

pub struct FileTransferService {
    proxy: EventLoopProxy<UserEvent>,
    queue: Arc<Mutex<VecDeque<FileTransferCmdStart>>>,
    running: Arc<Mutex<Option<Transfer>>>
}

struct Transfer(Sender<FileTransferCmdResponse>);

impl FileTransferService {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            proxy,
            queue: Arc::new(Mutex::new(VecDeque::new())),
            running: Arc::new(Mutex::new(None))
        }
    }

    pub fn enqueue(&self, cmd: FileTransferCmdStart) {
        self.queue.lock().unwrap().push_back(cmd);
        self.drain();
    }

    pub fn update(&self, response: FileTransferCmdResponse) {
        match self.running.lock().unwrap().as_mut() {
            Some(Transfer(sender)) => sender.send(response).unwrap(),
            _ => {}
        }
    }

    fn drain(&self) {
        if self.running.lock().unwrap().is_some() {
            return;
        }

        let proxy = self.proxy.clone();
        let queue = self.queue.clone();
        let running = self.running.clone();

        thread::spawn(move || {
            loop {
                let next = queue.lock().unwrap().pop_front();
                match next {
                    Some(cmd) => {
                        let proxy = proxy.clone();
                        let (sender, receiver) = mpsc::channel::<FileTransferCmdResponse>();
                        let handle = spawn_file_transfer(cmd, receiver, proxy);
                        running.lock().unwrap().replace(Transfer(sender));
                        handle.join().unwrap();
                    },
                    None => {
                        break;
                    }
                }
            }
            *running.lock().unwrap() = None;
        });
    }
}

fn spawn_file_transfer(
    cmd: FileTransferCmdStart,
    rec: Receiver<FileTransferCmdResponse>,
    proxy: EventLoopProxy<UserEvent>) -> JoinHandle<()>
{
    let options = CopyOptions::default();

    thread::spawn(move || match cmd {
        FileTransferCmdStart::Copy { parent, names, to } => {
            let from: Vec<_> = names.into_iter().map(|name| parent.join(name)).collect();

            copy_items_with_progress(&from, &to, &options, move |progress| {
                let event =
                    UserEvent::FileTransferProgress(FileTransferProgress::from(progress));

                proxy.send_event(event).unwrap();
                rec.recv().unwrap().into()
            }).unwrap();
        },
        FileTransferCmdStart::Cut { parent, names, to } => {
            let from: Vec<_> = names.into_iter().map(|name| parent.join(name)).collect();

            move_items_with_progress(&from, &to, &options, move |progress| {
                let event =
                    UserEvent::FileTransferProgress(FileTransferProgress::from(progress));

                proxy.send_event(event).unwrap();
                rec.recv().unwrap().into()
            }).unwrap();
        }
    })
}
