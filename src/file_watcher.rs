use wry::application::event_loop::EventLoopProxy;
use crate::UserEvent;
use notify::{RecursiveMode,Watcher,RecommendedWatcher};
use notify_debouncer_mini::{new_debouncer,Debouncer,DebounceEventResult};
use std::path::{PathBuf,Path};
use std::sync::mpsc::Receiver;
use std::time::Duration;
use crate::{Location,Options};
use std::sync::{Arc,Mutex};
use std::thread;

pub struct FileWatcher {
    debouncer: Arc<Mutex<Debouncer<RecommendedWatcher>>>,
    watching: Arc<Mutex<Option<PathBuf>>>,
    proxy: EventLoopProxy<UserEvent>,
    receiver: Receiver<DebounceEventResult>,
    location: Location
}

impl FileWatcher {
    pub fn new(location: Location, proxy: EventLoopProxy<UserEvent>) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let debouncer = new_debouncer(Duration::from_secs(2), None, tx).unwrap();

        Self {
            debouncer: Arc::new(Mutex::new(debouncer)),
            proxy,
            receiver: rx,
            location,
            watching: Arc::new(Mutex::new(None)),
        }
    }

    pub fn watch(&self, path: PathBuf, options: &Options) {
        let mut watching = self.watching.lock().unwrap().clone();
        let mut debouncer = self.debouncer.lock().unwrap();

        if let Some(existing) = watching.clone() {
            debouncer.watcher().unwatch(&existing).unwrap();
        }

        debouncer
            .watcher()
            .watch(Path::new("."), RecursiveMode::NonRecursive)
            .unwrap();

        *self.watching.lock().unwrap() = Some(path.clone());

        thread::spawn(move || {
            for result in self.receiver.iter() {
                let folder = self.location.update(&path, &options);
                self.proxy.send_event(UserEvent::UpdateFolder {
                    folder,
                    script_result: None
                });
            }
        });
    }
}
