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

        // libgit2 do not support abs path
        let t_obj;
        let walk_start_point = if path == Path::new("/") {
             &root_tree
        } else {
            let path_for_git = if path.starts_with("/") {
                path.strip_prefix("/")?
            } else {
                path
            };
            t_obj = root_tree.get_path(path_for_git)?.to_object(&self.git_repo)?;
            match t_obj.as_tree(){
                Some(t) => t,
                None => return Ok(()),
            }
        };
        
        walk_start_point.walk(TreeWalkMode::PreOrder, |root_rel_path, entry| {
            let filename =  match entry.name(){
                Some(t) => t,
                None => return 0,
            };
            let abs_path = path.to_owned().join(root_rel_path).join(filename);
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
    let ver = backend.get_current_version().unwrap();
    println!("{}", ver.name);
    println!("{:?}", ver.id);

    let cb = &mut |d:&DirItem| {
        println!("{:?} == {:?}", d.abs_path, d.hash);
        Ok(WalkRetCtl::Next)
    };

    backend.walk_dir(&ver, &Path::new("/test/mock_data"), cb).unwrap();
    
}