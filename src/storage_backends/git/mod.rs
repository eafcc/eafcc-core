use crate::{error::{StorageBackendError, WalkDirError}, model::object::{ObjectID, ObjectIDRef}};
use core::time;
use std::{cell::Cell, collections::HashMap, env, fs, path::{Path, PathBuf}, str::{FromStr, Utf8Error}, sync::{Arc, mpsc::{channel, Receiver}}};

use super::{DirItem, StorageBackend, StorageChangeEvent, VersionItem, WalkRetCtl};
use super::Result;
use std::str;
use std::thread;
use std::time::Duration;
use git2::{Oid, Remote, Repository, TreeEntry, TreeWalkMode};
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

		let git_repo = match Repository::open_bare(local_repo_path.clone()) {
            Ok(t) => t,
            Err(_) => {
                // let new_repo = Repository::init_bare(local_repo_path)?;
                // new_repo.remote_add_fetch("origin", &remote_repo_url)?;
                let fetch_opts = get_fetch_opt();
                let new_repo = git2::build::RepoBuilder::new().bare(true).fetch_options(fetch_opts).clone(&remote_repo_url, &local_repo_path)?;
            
                new_repo
            }
        };
        let ret = Self(Arc::new(RwLock::new(GitBackendInner{
            local_repo_path,
            remote_repo_url:remote_repo_url.clone(),
			git_repo: Arc::new(ReentrantMutex::new(git_repo)),
            target_branch_name:target_branch_name.clone(),
            cur_version: VersionItem{
                name: "".to_string(),
                id: ObjectID::new(),
            },
        })));


        git_sync_local_branch(&remote_repo_url,&ret.0, &target_branch_name)?;

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
        
        match git_sync_local_branch(&remote_repo_url, &backend_inner, &branch_name){
            Ok(Some(new_version)) => {
                cb(StorageChangeEvent{
                    new_version: new_version.clone(),
                });
                
                let mut backend_inner_guard = backend_inner.write();
                backend_inner_guard.cur_version = new_version;
            },
            _ => {},
        };
    }
}




fn git_sync_local_branch(remote_repo_url: &String, backend_inner: &Arc<RwLock<GitBackendInner>>, branch_name: &String) -> Result<Option<VersionItem>>{

    let mut tmp_remote_probe = match git2::Remote::create_detached(remote_repo_url.as_str()) {
        Err(e) => {
            print_error_with_switch!("git watch error when create temp client to remote git repo: {:?}", e);
            return Err(StorageBackendError::Git2Error(e)) 
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
    match tmp_remote_probe.connect_auth(git2::Direction::Fetch, Some(callbacks),None) {
        Err(e) => {
            print_error_with_switch!("git watch error when connect to remote git repo: {:?}", e);
            return Err(StorageBackendError::Git2Error(e)) 
        }, 
        _ => {},
    };

    let remote_heads = match tmp_remote_probe.list(){
        Err(e) => {
            print_error_with_switch!("git update monitor get list error {:?}", e);
            return Err(StorageBackendError::Git2Error(e)) 
        }, 
        Ok(t) => t,
    };  
    
    let branch_name_in_refspec_format = "refs/heads/".to_owned() + &branch_name;
    let backend_inner_guard = backend_inner.read();
    let mut newest_remote_commit_id = None;
    for remote_head in remote_heads {
        if remote_head.name() != branch_name_in_refspec_format {
            continue
        }
        if remote_head.oid().as_bytes() == backend_inner_guard.cur_version.id {
            return Ok(None)
        }

        newest_remote_commit_id = Some(remote_head.oid().clone());
        break
    }

    drop(backend_inner_guard);
    let backend_inner_guard = backend_inner.write();
    let git_repo = backend_inner_guard.git_repo.lock();

    let mut remote = match git_repo.find_remote("origin") {
        Err(e) => {
            print_error_with_switch!("git update monitor build fetch remote error {:?}", e);
            return Err(StorageBackendError::Git2Error(e))
        }, 
        Ok(t) => t,
    };

    let mut fetch_opts = get_fetch_opt();
    match remote.fetch (&[&branch_name_in_refspec_format], Some(&mut fetch_opts), Some("hahahah")){
        Err(e) => {
            print_error_with_switch!("git update monitor fetch data error {:?}", e);
            return Err(StorageBackendError::Git2Error(e))
        }, 
        Ok(t) => t,
    };
    
    let new_commit = match git_repo.find_commit(newest_remote_commit_id.unwrap()){
        Err(e) => {
            print_error_with_switch!("git update monitor get remote commit object error {:?}", e);
            return Err(StorageBackendError::Git2Error(e))
        }, 
        Ok(t) => t,
    };
    match git_repo.branch(&backend_inner_guard.target_branch_name, &new_commit, true){
        Err(e) => {
            print_error_with_switch!("git update monitor change branch pointing to error {:?}", e);
            return Err(StorageBackendError::Git2Error(e))
        }, 
        Ok(t) => t,
    };

    let new_version = VersionItem{
        name: new_commit.message().unwrap_or_default().to_string(),
        id: new_commit.id().as_bytes().to_vec(),
    };

    return Ok(Some(new_version))
    
}



fn get_fetch_opt() -> git2::FetchOptions<'static> {
    let mut fetch_opts = git2::FetchOptions::new();
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
    fetch_opts.remote_callbacks(callbacks);
    fetch_opts
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