pub mod filesystem;
use std::io::Result;

use crate::model::object::{ObjectID, ObjectIDRef};


pub struct StorageChangeEvent {
	old_version: String,
	new_version: String,
	path: String,
}

pub struct DirItem {
	// if the return value end with "/", then it has children(a dir), otherwise, it's a leaf node (a file) 
	// only return current node name, never include parent's name. i.e. if it is a dir, the it should not have "/" as prefix and
	// must have a "/" as suffix
	pub name: String,

	// when this is an dir, hash can be empty string now. if we are using a backend like git, then it's tree object does have a hash
	// but for backend like noraml filesytem, it's hard to define hash on a folder.
	pub hash: ObjectID,
}

pub trait StorageBackend {
	fn get_obj_by_hash(&self, hash: ObjectIDRef) -> Result<Vec<u8>>;
	fn list_dir(&self, version: &str, path: &str) -> Result<Vec<DirItem>>;
	fn get_hash_by_path(&self, version: &str, path: &str) -> Result<ObjectID>;
	fn set_update_cb(&mut self, cb: fn(Vec<StorageChangeEvent>));
}