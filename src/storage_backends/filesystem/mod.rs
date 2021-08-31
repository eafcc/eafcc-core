#![feature(shared_from_slice)]
use crate::model::object::{ObjectID, ObjectIDRef};
use std::{cell::{Cell, RefCell}, collections::HashMap, fs, path::{Path, PathBuf}, sync::{Arc, mpsc::channel}};

use super::{DirItem, StorageBackend, StorageChangeEvent};
use notify::{watcher, RecommendedWatcher, RecursiveMode, Watcher};
use std::io::Result;
use std::str;
use std::thread;
use std::time::Duration;

/*
Warning:
This backend is for develop and testing, Never use this backend in production, because it has at least the following problems:

* the full path and filename is use as a 'hash', use this trick to locate a file in the storage
* there is no version control
* it only support *nix system, no windoes support

*/

pub struct FilesystemBackend {
    hash_2_path: HashMap<ObjectID, PathBuf>,
    base_path: PathBuf,
    cb: RefCell<Option<Arc<dyn Fn(Vec<StorageChangeEvent>) + Send + Sync >>>,
}

impl FilesystemBackend {
    pub fn new(base_path: PathBuf) -> FilesystemBackend {
        let ret = Self {
            hash_2_path: HashMap::new(),
            base_path,
            cb: RefCell::new(None),
        };
        return ret;
    }
}

impl StorageBackend for FilesystemBackend {
    fn get_obj_by_hash(&self, hash: ObjectIDRef) -> Result<Vec<u8>> {
        let path = Path::new(
            str::from_utf8(hash)
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "invalid path"))?,
        );
        fs::read(path)
    }

    fn list_dir(&self, version: &str, path: &str) -> Result<Vec<DirItem>> {
        let mut ret = Vec::new();

        let path = if path.starts_with("/") {
            self.base_path.join(&path[1..])
        } else {
            self.base_path.join(path)
        };

        for t in fs::read_dir(path)? {
            let path = t?.path();

            if let Some(f) = path.file_name() {
                let mut f = f
                    .to_os_string()
                    .into_string()
                    .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "invalid path"))?;
                if path.is_dir() {
                    f.push_str("/")
                }
                ret.push(DirItem {
                    name: f,
                    hash: Vec::from(path.to_str().ok_or(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "invalid path",
                    ))?),
                });
            }
        }

        return Ok(ret);
    }

    fn get_hash_by_path(&self, version: &str, path: &str) -> Result<ObjectID> {
        let path = if path.starts_with("/") {
            self.base_path.join(&path[1..])
        } else {
            self.base_path.join(path)
        };
        if let Ok(m) = fs::metadata(&path) {
            if m.is_file() {
                return Ok(Vec::from(path.to_str().ok_or(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "invalid path",
                ))?));
            }
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "not exist or not a file",
        ));
    }

    fn set_update_cb(&self, cb: Box<dyn Fn(Vec<StorageChangeEvent>) + Send + Sync>) {
        if self.cb.borrow().is_none() {
            let t:Arc<dyn Fn(Vec<StorageChangeEvent>) + Send + Sync> = Arc::from(cb);
            self.cb.replace(Some(t));
            let path = self.base_path.to_string_lossy().to_string();
            let cb = self.cb.borrow().clone().unwrap();
            thread::spawn(move || {
                eafcc_watcher(
                    path,
                    cb,
                )
            });
        }
    }
}


fn eafcc_watcher(path: String, cb: Arc<dyn Fn(Vec<StorageChangeEvent>)>) {
    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Create a watcher object, delivering debounced events.
    // The notification back-end is selected based on the platform.
    let mut watcher = watcher(tx, Duration::from_secs(10)).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(&path, RecursiveMode::Recursive).unwrap();

    loop {
        match rx.recv() {
            Ok(event) => {
                cb(Vec::new());
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}