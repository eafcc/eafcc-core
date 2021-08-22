
pub struct StorageChangeEvent {
	old_version: String,
	new_version: String,
	path: String,
}

pub trait StorageBackend {
	fn get_obj_by_path(version: &str, path: &str) -> Vec<u8>;
	fn get_obj_by_hash(hash: &str) -> Vec<u8>;
	fn list_dir(version: &str, path: &str) -> Vec<String>;
	fn set_update_cb(cb: fn(Vec<StorageChangeEvent>));

}