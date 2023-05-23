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

pub struct FileTransferService {
    proxy: EventLoopProxy<UserEvent>,
    queue: Arc<Mutex<Vec<FileTransfer>>>
}

impl FileTransferService {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            proxy,
            queue: Arc::new(Mutex::new(vec![]))
        }
    }

    pub fn enqueue(&self, cmd: FileTransferCmdStart) {
        let mut queue = self.queue.lock().unwrap();

        if queue.len() == 0 {
            let (snd, rec) = std::sync::mpsc::channel::<FileTransferCmdResponse>();
            let proxy = self.proxy.clone();
            let options = CopyOptions::default();
            let handle = thread::spawn(move || match cmd {
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
            });

            queue.push(FileTransfer::Running((snd, handle)));
        } else {
            queue.push(FileTransfer::Queued(cmd));
        }
    }

    pub fn update(&self, response: FileTransferCmdResponse) {
        let queue = self.queue.lock().unwrap();

        match queue.first() {
            Some(FileTransfer::Running((snd, _))) => snd.send(response).unwrap(),
            _ => unreachable!()
        }
    }
}

enum FileTransfer {
    Queued(FileTransferCmdStart),
    Running((Sender<FileTransferCmdResponse>, JoinHandle<()>))
}
