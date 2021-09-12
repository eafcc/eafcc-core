use crate::model::object::{ObjectID, ObjectIDRef};
use std::{cell::Cell, collections::HashMap, fs, path::{Path, PathBuf}, sync::{Arc, Mutex, mpsc::channel}};

use super::{DirItem, StorageBackend, StorageChangeEvent};
use notify::{watcher, RecursiveMode, Watcher, DebouncedEvent};
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
}

impl FilesystemBackend {
    pub fn new(base_path: PathBuf) -> FilesystemBackend {
        let ret = Self {
            hash_2_path: HashMap::new(),
            base_path,
        };
        return ret;
    }

    fn get_versioned_path(&self, version: &str, path: &str) -> PathBuf {
        let t = self.base_path.join(version);
        if path.starts_with("/") {
            t.join(&path[1..])
        } else {
            t.join(path)
        }
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

        let path = self.get_versioned_path(version, path);

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
        let path = self.get_versioned_path(version, path);
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

    fn set_update_cb(&self, cb: Box<dyn Fn(StorageChangeEvent) + Send + Sync>) {

        let path = self.base_path.join("head");

        let cb_inner = Box::new(move |new_version: String| {
            cb(StorageChangeEvent{
                new_version: new_version.clone(),
            });
        });
        thread::spawn(move || {
            eafcc_watcher(
                path,
                cb_inner,
            )
        });
        
    }

    fn get_diff_list(&self, old_version: &str, new_version: &str, namespace: &str) -> Result<Vec<String>>{
        return Ok(Vec::new())
    }

	fn get_current_version(&self) -> Result<String>{
        read_version_from_fs(&self.base_path.join("head"))
    }
	fn list_versions(&self) -> Result<Vec<String>>{
        return Ok(Vec::new())
    }
}

fn read_version_from_fs(path: &Path) -> Result<String>{
    let t = fs::read(path)?;
    Ok(String::from_utf8(t).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?)
}

fn eafcc_watcher(path: PathBuf, cb: Box<dyn Fn(String)>) {
    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Create a watcher object, delivering debounced events.
    // The notification back-end is selected based on the platform.
    let mut watcher = watcher(tx, Duration::from_secs(2)).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(&path, RecursiveMode::Recursive).unwrap();

    loop {
        match rx.recv() {
            Ok(event) => {
                match event {
                    DebouncedEvent::Chmod(_) | DebouncedEvent::Create(_) | DebouncedEvent::Remove(_) | DebouncedEvent::Rename(_,_) | DebouncedEvent::Rescan | DebouncedEvent::Write(_) => {
                        if let Ok(new_version) = read_version_from_fs(&path){
                            cb(new_version)
                        }
                    },
                    _ => continue,
                }
                
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}