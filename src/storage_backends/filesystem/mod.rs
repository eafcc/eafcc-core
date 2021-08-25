use std::{collections::HashMap, fs, path::{Path, PathBuf}};
use crate::model::object::{ObjectID, ObjectIDRef};

use super::{DirItem, StorageBackend, StorageChangeEvent};
use std::io::Result;
use std::str;
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
		return Self{
			hash_2_path: HashMap::new(),
			base_path,
		}
	}
}

impl StorageBackend for FilesystemBackend {
	fn get_obj_by_hash(&self, hash: ObjectIDRef) -> Result<Vec<u8>>{
		let path = Path::new(str::from_utf8(hash).map_err(|_|std::io::Error::new(std::io::ErrorKind::Other, "invalid path"))?);
		fs::read(path)
	}

	fn list_dir(&self, version: &str, path: &str) -> Result<Vec<DirItem>>{
		let mut ret = Vec::new();
		
		let path = if path.starts_with("/") {
			self.base_path.join(&path[1..])
		} else {
			self.base_path.join(path)
		};
		 

		for t in fs::read_dir(path)? {
			let path = t?.path();

			if let Some(f) = path.file_name() {
				let mut f = f.to_os_string().into_string().map_err(|_|std::io::Error::new(std::io::ErrorKind::Other, "invalid path"))?;
				if path.is_dir() {
					f.push_str("/")
				}
				ret.push(DirItem{
					name: f,
					hash: Vec::from( path.to_str().ok_or(std::io::Error::new(std::io::ErrorKind::Other, "invalid path"))?),
				});
			}
			
		}

		return Ok(ret)
	}
	fn set_update_cb(&mut self, cb: fn(Vec<StorageChangeEvent>)){
		
	}
}

