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
use crate::{FileTransfer,FileTransferKind,FileTransferCmd,FileTransferCmdStart,FileTransferCmdResponse,FileTransferProgress,FileTransferProgressState,UserEvent};
use std::collections::VecDeque;
use log;

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
        log::trace!("File transfer queued -> {cmd:#?}");
        self.queue.lock().unwrap().push_back(cmd);
        self.drain();
    }

    pub fn update(&self, response: FileTransferCmdResponse) {
        log::trace!("File transfer update -> {response:#?}");

        match self.running.lock().unwrap().as_mut() {
            Some(Transfer(sender)) => sender.send(response).unwrap(),
            _ => log::error!("File transfer update doesn't have a transfer target")
        }
    }

    fn drain(&self) {
        if self.running.lock().unwrap().is_some() {
            log::trace!("File transfer already running");
            return;
        }

        let proxy = self.proxy.clone();
        let queue = self.queue.clone();
        let running = self.running.clone();

        thread::spawn(move || {
            log::trace!("File transfer starting");
            loop {
                let next = queue.lock().unwrap().pop_front();
                match next {
                    Some(cmd) => {
                        let proxy = proxy.clone();
                        let (sender, receiver) = mpsc::channel::<FileTransferCmdResponse>();
                        let handle = spawn_file_transfer(cmd, receiver, proxy);
                        running.lock().unwrap().replace(Transfer(sender));
                        handle.join().unwrap();
                        log::trace!("File transfer finished");
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
    let options = CopyOptions::new();

    thread::spawn(move || {
        let from: Vec<_> = cmd.names.into_iter().map(|name| cmd.parent.join(name)).collect();
        let mut file_transfer = FileTransfer {
            state: Default::default(),
            progress: Default::default(),
            from: cmd.parent,
            to: cmd.to.clone(),
            kind: cmd.kind
        };

        let on_progress = |progress: fs_extra::TransitProcess| {
            file_transfer.state = progress.state.clone().into();
            file_transfer.progress = progress.into();
            proxy.send_event(UserEvent::FileTransferProgress(file_transfer.clone())).unwrap();
            rec.recv().unwrap().into()
        };

        let result = match cmd.kind {
            FileTransferKind::Cut => move_items_with_progress(&from, &cmd.to, &options, on_progress),
            FileTransferKind::Copy => copy_items_with_progress(&from, &cmd.to, &options, on_progress),
        };

        if let Err(e) = result {
            log::error!("Error transfering files: {e:?}");
        }

        file_transfer.state = FileTransferProgressState::Finished;
        proxy.send_event(UserEvent::FileTransferProgress(file_transfer)).unwrap();
    })
}
