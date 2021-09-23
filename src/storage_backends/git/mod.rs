use crate::{error::{StorageBackendError, WalkDirError}, model::object::{ObjectID, ObjectIDRef}};
use core::time;
use std::{cell::Cell, collections::HashMap, env, fs, path::{Path, PathBuf}, str::{FromStr, Utf8Error}, sync::{Arc, mpsc::{channel, Receiver}}};

use super::{DirItem, StorageBackend, StorageChangeEvent, VersionItem, WalkRetCtl};
use super::Result;
use std::str;
use std::thread;
use std::time::Duration;
use git2::{Oid, Repository, TreeEntry, TreeWalkMode};
use parking_lot::{ReentrantMutex, RwLock};


pub struct GitBackend(Arc<RwLock<GitBackendInner>>);
pub struct GitBackendInner {
    local_repo_path: PathBuf,
    remote_repo_url: String,
    git_repo: Arc<ReentrantMutex<Repository>>,
    target_branch_name: String,
    cur_version: VersionItem,
}

impl GitBackend {
    pub fn new(local_repo_path: PathBuf, remote_repo_url: String, target_branch_name: String) -> Result<GitBackend> {

		let git_repo = Repository::open_bare(local_repo_path.clone())?;
        let ret = Self(Arc::new(RwLock::new(GitBackendInner{
            local_repo_path,
            remote_repo_url,
			git_repo: Arc::new(ReentrantMutex::new(git_repo)),
            target_branch_name,
            cur_version: VersionItem{
                name: "".to_string(),
                id: ObjectID::new(),
            },
        })));

        let cur_version = ret.get_current_version()?;
        ret.0.write().cur_version = cur_version;
        return Ok(ret);
    }
}

impl StorageBackend for GitBackend {
    fn get_obj_by_hash(&self, hash: ObjectIDRef) -> Result<Vec<u8>> {
        let backend_inner = self.0.read();
        let git_repo = backend_inner.git_repo.lock();
        let git_oid = git2::Oid::from_bytes(hash)?;
        let blob = git_repo.find_blob(git_oid)?;
        Ok(Vec::from(blob.content()))
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
        let backend_inner = self.0.read();
        let git_repo = backend_inner.git_repo.lock();
        let commit = git_repo.find_commit(oid)?;
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
            t_obj = root_tree.get_path(path_for_git)?.to_object(&git_repo)?;
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
        let backend_inner = self.0.read();
        let repo_cloned = backend_inner.git_repo.clone();
        let remote_repo_url_cloned = backend_inner.remote_repo_url.clone();
        let backend_inner_cloned = self.0.clone();
        let branch_name_clone = backend_inner.target_branch_name.clone();
        thread::spawn(move || {
            git_watcher(remote_repo_url_cloned, backend_inner_cloned, branch_name_clone, cb);
        });
        return Ok(())
    }

    fn get_diff_list(&self, old_version: &VersionItem, new_version: &VersionItem, namespace: &str) -> Result<Vec<String>>{
        return Ok(Vec::new())
    }

	fn get_current_version(&self) -> Result<VersionItem>{
        let backend_inner = self.0.read();
        let git_repo = backend_inner.git_repo.lock();
        let branch = git_repo.find_branch(&backend_inner.target_branch_name, git2::BranchType::Local)?;
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



fn git_watcher(remote_repo_url: String, backend_inner: Arc<RwLock<GitBackendInner>>, branch_name: String, cb: Box<dyn Fn(StorageChangeEvent) + Send + Sync>) {
    loop {
        thread::sleep(time::Duration::from_secs(2));
        let mut remote = match git2::Remote::create_detached(remote_repo_url.as_str()) {
            Err(e) => {
                print_error_with_switch!("git watch error when create temp client to remote git repo: {:?}", e);
                continue
            }, 
            Ok(t) => t,
        };

        let mut callbacks = git2::RemoteCallbacks::new();
        // TODO do the cert check
        callbacks.certificate_check(|_, _| {true});
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            git2::Cred::ssh_key(
              username_from_url.unwrap(),
              None,
              std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
              None,
            )
          });

        // TODO timeout control
        match remote.connect_auth(git2::Direction::Fetch, Some(callbacks),None) {
            Err(e) => {
                print_error_with_switch!("git watch error when connect to remote git repo: {:?}", e);
                continue
            }, 
            _ => {},
        };

        let remote_heads = match remote.list(){
            Err(e) => {
                print_error_with_switch!("git update monitor get list error {:?}", e);
                continue
            }, 
            Ok(t) => t,
        };  
        
        let branch_name_in_refspec_format = "refs/heads/".to_owned() + &branch_name;
        for remote_head in remote_heads {
            println!("listed_name --> {}", remote_head.name() )   ;
            if remote_head.name() != branch_name_in_refspec_format {
                continue
            }
            println!("remote_oid: {:?}", remote_head.oid());
            println!("local_oid: {:?}", backend_inner.read().cur_version.id);
            
        }

        // match remote.fetch(&[branch_name.as_ref() as &str], None, None){
            
        // };
    }
}




#[test]
fn test_git_backend_smoke() {
    let project_base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let local_repo_path = project_base_dir
        .join(".git");
    let mut backend = Box::new(GitBackend::new(local_repo_path, "".to_string(),"master".to_string()).unwrap());
    let ver = backend.get_current_version().unwrap();
    println!("{}", ver.name);
    println!("{:?}", ver.id);

    let cb = &mut |d:&DirItem| {
        println!("{:?} == {:?}", d.abs_path, d.hash);
        Ok(WalkRetCtl::Next)
    };

    backend.walk_dir(&ver, &Path::new("/test/mock_data"), cb).unwrap();
    
}