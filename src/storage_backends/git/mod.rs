use crate::{error::{StorageBackendError, WalkDirError}, model::object::{ObjectID, ObjectIDRef}};
use std::{cell::Cell, collections::HashMap, fs, path::{Path, PathBuf}, str::{FromStr, Utf8Error}, sync::{Arc, Mutex, mpsc::{channel, Receiver}}};

use super::{DirItem, StorageBackend, StorageChangeEvent, VersionItem, WalkRetCtl};
use super::Result;
use std::str;
use std::thread;
use std::time::Duration;
use git2::{Oid, Repository, TreeEntry, TreeWalkMode};


pub struct GitBackend {
    base_path: PathBuf,
    git_repo: Repository,
    target_branch_name: String,
}

impl GitBackend {
    pub fn new(base_path: PathBuf, target_branch_name: String,) -> Result<GitBackend> {

		let git_repo = Repository::open_bare(base_path.clone())?;
        let ret = Self {
            base_path,
			git_repo,
            target_branch_name,
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

    fn list_dir(&self, version: &VersionItem, path: &Path) -> Result<Vec<DirItem>> {
		// TODO 
        let mut ret = Vec::new();
        return Ok(ret);
    }

	fn walk_dir(
        &self,
        version: &VersionItem,
        path: &Path,
        cb: &mut dyn FnMut(&DirItem) -> std::result::Result<WalkRetCtl, WalkDirError>,
    ) -> Result<()> {
        let oid = Oid::from_bytes(&version.id)?;
        let commit = self.git_repo.find_commit(oid)?;
        let root_tree = commit.tree()?;
        let walk_start_point = root_tree.get_path(path)?;
        if let Some(tree) = walk_start_point.to_object(&self.git_repo)?.as_tree() {
            tree.walk(TreeWalkMode::PreOrder, |filename, entry| {
                let abs_path =  PathBuf::from_str(match entry.name(){
                    Some(t) => t,
                    None => return 0,
                }).unwrap();
                let hash = Vec::from(entry.id().as_bytes());
                let is_dir = match entry.kind() {
                    Some(git2::ObjectType::Tree) => true,
                    _ => false,
                };
                let dir_item = DirItem::new(abs_path, is_dir, hash);
                match cb(&dir_item) {
                    Ok(t) => match t {
                        WalkRetCtl::Next => 0,
                        WalkRetCtl::SkipCurrentNode => 1,
                        WalkRetCtl::StopWalking => -1,
                    },
                    Err(_) => 0,
                }
            })?;
        }

        return Ok(());
    }

    fn get_hash_by_path(&self, version: &VersionItem, path: &Path) -> Result<ObjectID> {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "not exist or not a file",
        ))?;
    }

    fn set_update_cb(&self, cb: Box<dyn Fn(StorageChangeEvent) + Send + Sync>) -> Result<()> {
        return Ok(())
    }

    fn get_diff_list(&self, old_version: &VersionItem, new_version: &VersionItem, namespace: &str) -> Result<Vec<String>>{
        return Ok(Vec::new())
    }

	fn get_current_version(&self) -> Result<VersionItem>{
        let branch = self.git_repo.find_branch(&self.target_branch_name, git2::BranchType::Local)?;
        let commit = branch.into_reference().peel_to_commit()?;
        let name = str::from_utf8(commit.message_bytes())?;
        let commit_id = commit.id();
        let oid = commit_id.as_bytes();
        Ok(VersionItem{
            name: name.to_string(),
            id: ObjectID::from(oid),
        })

    }
	fn list_versions(&self, start: usize, limit: usize) -> Result<Vec<VersionItem>>{
        return Ok(Vec::new())
    }
}



#[test]
fn test_git_backend_smoke() {
    let project_base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let base_path = project_base_dir
        .join(".git");
    let mut backend = Box::new(GitBackend::new(base_path, "master".to_string()).unwrap());
    let t = backend.get_current_version().unwrap();
    println!("{}", t.name);
    println!("{:?}", t.id);
    
}