use std::{default, ffi::CStr};

use libc::c_void;
use libgit2_sys::{self as raw, git_oid, git_odb_backend, GIT_OK};
use git2::{Oid, Remote, Repository, TreeEntry, TreeWalkMode, Odb};


#[repr(C)]
struct P2PBackend {
	parent: raw::git_odb_backend,
}



extern "C" fn backend_read(data_p: *mut *mut c_void, len_p: *mut usize, type_p: *mut i32, _backend: *mut raw::git_odb_backend, oid: *const raw::git_oid) -> i32 {
	println!("hello........read");
	GIT_OK
} 


extern "C" fn backend_read_prefix(out_oid: *mut git_oid, data_p: *mut *mut c_void, len_p: *mut usize, type_p: *mut i32, _backend: *mut raw::git_odb_backend, short_oid: *const raw::git_oid, len: usize) -> i32 {
	println!("hello........read_prefix");
	GIT_OK
}


extern "C" fn backend_exists(_backend: *mut git_odb_backend, oid: *const git_oid) -> i32 {
	println!("hello........read_exists");
	GIT_OK
}

extern "C" fn backend_read_header(len_p: *mut usize, type_p: *mut i32, _backend: *mut git_odb_backend, oid: *const git_oid) -> i32 {
	println!("hello........read_header");
	GIT_OK
}





#[test]
fn test_change_custom_backend() {

	let mut odb = Odb::new().unwrap();
	// odb.add_new_mempack_backend(500).unwrap();

	let odb_inner = &mut odb as *mut Odb as *mut *mut raw::git_odb;
	
	let odb_raw = unsafe{*odb_inner};
	


	let mut parent = raw::git_odb_backend {
		version: raw::GIT_ODB_BACKEND_VERSION,
		odb: odb_raw,
		read: Some(backend_read),
		read_prefix: Some(backend_read_prefix),
		read_header: Some(backend_read_header),
		write: None,
		writestream: None,
		readstream: None,
		exists: Some(backend_exists),
		exists_prefix: None,
		refresh: None,
		foreach: None,
		writepack: None,
		writemidx: None,
		freshen: None,
		free: None,
	};

	// let mut backend = P2PBackend {parent};
	// let mut backend = &mut backend as *mut P2PBackend as *mut raw::git_odb_backend;

	let t = &mut parent as *mut raw::git_odb_backend;

	unsafe{
		let ret = raw::git_odb_add_backend(odb_raw, t ,1000);

		// let s = CStr::from_ptr((*raw::git_error_last()).message);


		// println!("---->{}", s.to_str().unwrap());
	}

	
	let repo = Repository::from_odb(odb).unwrap();

	
	repo.find_blob(Oid::from_str("aabbccdd").unwrap()).unwrap();
	

}