pub mod filesystem;
// pub mod git;

use std::path::{Path, PathBuf};

use crate::{
    error::{CCLibError, StorageBackendError, WalkDirError},
    model::object::{ObjectID, ObjectIDRef},
};

type Result<T> = std::result::Result<T, StorageBackendError>;

pub struct StorageChangeEvent {
    pub new_version: String,
}

pub struct DirItem {
    pub abs_path: PathBuf,
    is_dir: bool,

    // when this is an dir, hash can be empty string now. if we are using a backend like git, then it's tree object does have a hash
    // but for backend like noraml filesytem, it's hard to define hash on a folder.
    pub hash: ObjectID,
}

impl DirItem {
    pub fn new(abs_path: PathBuf, is_dir: bool, hash: ObjectID) -> Self {
        Self {
            abs_path,
            is_dir,
            hash,
        }
    }

	pub fn is_dir(&self) -> bool {
		return self.is_dir
	}
}

pub enum WalkRetCtl {
    Next,
    SkipCurrentNode,
    StopWalking,
}

pub trait StorageBackend {
    fn get_obj_by_hash(&self, hash: ObjectIDRef) -> Result<Vec<u8>>;
    fn list_dir(&self, version: &str, path: &Path) -> Result<Vec<DirItem>>;
    fn get_hash_by_path(&self, version: &str, path: &Path) -> Result<ObjectID>;
    fn set_update_cb(
        &self,
        cb: Box<dyn Fn(StorageChangeEvent) + Send + Sync + 'static>,
    ) -> Result<()>;
    fn get_diff_list(
        &self,
        old_version: &str,
        new_version: &str,
        namespace: &str,
    ) -> Result<Vec<String>>;
    fn get_current_version(&self) -> Result<String>;
    fn list_versions(&self) -> Result<Vec<String>>;

    fn walk_dir(
        &self,
        version: &str,
        path: &Path,
        cb: &mut dyn FnMut(&DirItem) -> std::result::Result<WalkRetCtl, WalkDirError>,
    ) -> Result<()> {
        let mut nodes_to_visit: Vec<PathBuf> = Vec::with_capacity(32);
        nodes_to_visit.push(path.to_owned());
        while let Some(ref parent_node) = nodes_to_visit.pop() {
            for cur_node in self.list_dir(version, &parent_node)? {
                match cb(&cur_node) {
                    Ok(t) => match t {
                        WalkRetCtl::StopWalking => return Ok(()),
                        WalkRetCtl::SkipCurrentNode => continue,
                        WalkRetCtl::Next => {
                            if cur_node.is_dir() {
                                nodes_to_visit.push(cur_node.abs_path);
                            }
                        }
                    },
                    Err(e) => return Err(StorageBackendError::WalkDirError(Box::new(e))),
                }
            }
        }
        return Ok(());
    }
}
