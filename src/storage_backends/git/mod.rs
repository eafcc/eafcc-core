use crate::{error::StorageBackendError, model::object::{ObjectID, ObjectIDRef}};
use std::{cell::Cell, collections::HashMap, fs, path::{Path, PathBuf}, sync::{Arc, Mutex, mpsc::{channel, Receiver}}};

use super::{DirItem, StorageBackend, StorageChangeEvent};
use super::Result;
use std::str;
use std::thread;
use std::time::Duration;
use git2::Repository;


pub struct GitBackend {
    base_path: PathBuf,
    git_repo: Repository,
}

impl GitBackend {
    pub fn new(base_path: PathBuf) -> Result<GitBackend> {

		let git_repo = Repository::open_bare(base_path)?;
        let ret = Self {
            base_path,
			git_repo,
        };
        return Ok(ret);
    }
}

impl StorageBackend for GitBackend {
    fn get_obj_by_hash(&self, hash: ObjectIDRef) -> Result<Vec<u8>> {
        let path = Path::new(
            str::from_utf8(hash)
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "invalid path"))?,
        );
        Ok(fs::read(path)?)
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
        ))?;
    }

    fn set_update_cb(&self, cb: Box<dyn Fn(StorageChangeEvent) + Send + Sync>) -> Result<()> {

        let path = self.base_path.join("head");

        let cb_inner = Box::new(move |new_version: String| {
            cb(StorageChangeEvent{
                new_version: new_version.clone(),
            });
        });

        let (tx, rx) = channel();

        // We use PollWatcher because we may use nfs so inotify maybe not work when file changed is triggered by some remote machine.
        // On the other hand, we only monitor a single file, so there won't be too mach overhead.
        let mut watcher = PollWatcher::new(tx, Duration::from_secs(2)).or(Err(StorageBackendError::UpdateWatchingError("error while setting up update watcher")))?;

        watcher.watch(&path, RecursiveMode::Recursive).or(Err(StorageBackendError::UpdateWatchingError("error while setting up update watcher")))?;

        let path_for_closure = path.clone();

        *(self.watcher.lock().or(Err(StorageBackendError::UpdateWatchingError("error while setting up update watcher")))?) = Some(watcher);

        thread::spawn(move || {
            eafcc_watcher(
                rx,
                path_for_closure,
                cb_inner,
            )
        });
        return Ok(())
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
    Ok(String::from_utf8(t).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?.to_string())
}

fn eafcc_watcher(rx: Receiver<DebouncedEvent>, path: PathBuf, cb: Box<dyn Fn(String)>) {
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
            Err(e) => {
                print_error_with_switch!("watch error: {:?}", e);
            },
        }
    }
}