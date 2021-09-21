use crate::{
    error::{ListDirError, StorageBackendError},
    model::object::{ObjectID, ObjectIDRef},
};
use std::{
    cell::Cell,
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        mpsc::{channel, Receiver},
        Arc, Mutex,
    },
};

use super::{Result, VersionItem};
use super::{DirItem, StorageBackend, StorageChangeEvent};
use nom::AsBytes;
use notify::{watcher, DebouncedEvent, PollWatcher, RecursiveMode, Watcher};
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
    base_path: PathBuf,
    watcher: Mutex<Option<PollWatcher>>,
}

impl FilesystemBackend {
    pub fn new(base_path: PathBuf) -> FilesystemBackend {
        let ret = Self {
            base_path,
            watcher: Mutex::new(None),
        };
        return ret;
    }

    fn get_versioned_path(&self, version: &VersionItem, path: &Path) -> Result<PathBuf> {
        let t = self.base_path.join(str::from_utf8(version.id.as_bytes())?);
        Ok(if path.starts_with("/") {
            t.join(
                path.strip_prefix("/")
                    .expect("should not reacher here, path has prefix"),
            )
        } else {
            t.join(path)
        })
    }
}

impl StorageBackend for FilesystemBackend {
    fn get_obj_by_hash(&self, hash: ObjectIDRef) -> Result<Vec<u8>> {
        let path = Path::new(
            str::from_utf8(hash)
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "invalid path"))?,
        );
        Ok(fs::read(path)?)
    }

    fn list_dir(&self, version: &VersionItem, path: &Path) -> Result<Vec<DirItem>> {
        let mut ret = Vec::new();

        let real_fs_abs_path = self.get_versioned_path(version, path)?;

        for t in fs::read_dir(real_fs_abs_path)? {
            let new_fs_abs_path = t?.path();

            if let Some(filename) = new_fs_abs_path.file_name() {
                let t = path.to_owned().join(filename);
                ret.push(DirItem::new(
                    t,
                    new_fs_abs_path.is_dir(),
                    Vec::from(new_fs_abs_path.to_str().ok_or(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "invalid path",
                    ))?),
                ));
            }
        }
        return Ok(ret);
    }

    fn get_hash_by_path(&self, version: &VersionItem, path: &Path) -> Result<ObjectID> {
        let path = self.get_versioned_path(version, path)?;
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
        ))?;
    }

    fn set_update_cb(&self, cb: Box<dyn Fn(StorageChangeEvent) + Send + Sync>) -> Result<()> {
        let path = self.base_path.join("head");

        let cb_inner = Box::new(move |new_version: VersionItem| {
            cb(StorageChangeEvent {
                new_version: new_version.clone(),
            });
        });

        let (tx, rx) = channel();

        // We use PollWatcher because we may use nfs so inotify maybe not work when file changed is triggered by some remote machine.
        // On the other hand, we only monitor a single file, so there won't be too mach overhead.
        let mut watcher = PollWatcher::new(tx, Duration::from_secs(2)).or(Err(
            StorageBackendError::UpdateWatchingError("error while setting up update watcher"),
        ))?;

        watcher.watch(&path, RecursiveMode::Recursive).or(Err(
            StorageBackendError::UpdateWatchingError("error while setting up update watcher"),
        ))?;

        let path_for_closure = path.clone();

        *(self
            .watcher
            .lock()
            .or(Err(StorageBackendError::UpdateWatchingError(
                "error while setting up update watcher",
            )))?) = Some(watcher);

        thread::spawn(move || eafcc_watcher(rx, path_for_closure, cb_inner));
        return Ok(());
    }

    fn get_diff_list(
        &self,
        old_version: &VersionItem,
        new_version: &VersionItem,
        namespace: &str,
    ) -> Result<Vec<String>> {
        return Ok(Vec::new());
    }

    fn get_current_version(&self) -> Result<VersionItem> {
        read_version_from_fs(&self.base_path.join("head"))
    }
    fn list_versions(&self, start: usize, limit: usize) -> Result<Vec<VersionItem>> {
        return Ok(Vec::new());
    }
}

fn read_version_from_fs(path: &Path) -> Result<VersionItem> {
    let t = fs::read(path)?;
    let name = str::from_utf8(&t)?
        .trim()
        .to_string();
    let id = ObjectID::from(name.as_bytes());
    Ok(VersionItem{
        name,
        id
    })
    
}

fn eafcc_watcher(rx: Receiver<DebouncedEvent>, path: PathBuf, cb: Box<dyn Fn(VersionItem)>) {
    loop {
        match rx.recv() {
            Ok(event) => match event {
                DebouncedEvent::Chmod(_)
                | DebouncedEvent::Create(_)
                | DebouncedEvent::Remove(_)
                | DebouncedEvent::Rename(_, _)
                | DebouncedEvent::Rescan
                | DebouncedEvent::Write(_) => {
                    if let Ok(new_version) = read_version_from_fs(&path) {
                        cb(new_version)
                    }
                }
                _ => continue,
            },
            Err(e) => {
                print_error_with_switch!("watch error: {:?}", e);
            }
        }
    }
}
