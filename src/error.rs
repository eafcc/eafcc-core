use std::path::StripPrefixError;
use std::result;

use std::ffi::NulError;
use std::str::Utf8Error;
use std::io::Error as IOError;
use std::sync::PoisonError;
use serde_json::Error as SerdeError;

use git2::Error as Git2Error;

use serde_json;
use thiserror::Error;


pub type Result<T> = result::Result<T, CCLibError>;

// 顶级Error
#[derive(Error, Debug)]
pub enum CCLibError {
	#[error("{0}")]
	NamespaceError(&'static str),
	#[error("error when calling cffi: {0}")]
	FFIError(#[from] FFIError),
	#[error("error with storage backend: {0}")]
	StorageBackendError(#[from] StorageBackendError),
	#[error("error with inmemory index: {0}")]
	MemoryIndexError(#[from] MemoryIndexError),
}

#[derive(Error, Debug)]
pub enum DifferError {
	#[error("error when querying: {0}")]
	QueryError(#[from] QueryError),
}

#[derive(Error, Debug)]
pub enum QueryError {
	#[error("error when locking internal state")]
	GetLockError,
}

#[derive(Error, Debug)]
pub enum MemoryIndexError {
	#[error("error while building index: {0}")]
	DataLoaderError(#[from] DataLoaderError),
	#[error("error while building index: {0}")]
	StorageBackendError(#[from] StorageBackendError),
	#[error("error, namespace must in absolute form, i.e., start with `/`")]
	NamespaceNotAbsolutePath,
}



#[derive(Error, Debug)]
pub enum DataLoaderError {
	#[error("error parse `spec` part in config: {0}")]
	SpecParseError(String),
	#[error("error parse json in config: {0}")]
	UnmarshalError(#[from] SerdeError),
}


#[derive(Error, Debug)]
pub enum StorageBackendError {
	#[error("error while trying to watch config change: {0}")]	
	UpdateWatchingError(&'static str),
	#[error("error while doing IO: {0}")]	
	IOError(#[from] IOError),
	#[error("error while operating on git backend: {0}")]	
	Git2Error(#[from] Git2Error),
	#[error("error while operating on git backend: {0}")]	
	WalkDirError(#[from] Box<WalkDirError>),
	#[error("error while operating on git backend: {0}")]	
	ListDirError(#[from] ListDirError),
	#[error("error while converting string: {0}")]	
	Utf8Error(#[from] Utf8Error),
	#[error("error while internally converting path: {0}")]	
	StripPrefixError(#[from] StripPrefixError),
	#[error("error while internally getting lock")]	
	LockPosionedError,
}

#[derive(Error, Debug)]
pub enum ListDirError {
	#[error("prefix not match, some link in filesystem backend maybe wrong")]	
	PathPrefixHandlingError,
	
}

#[derive(Error, Debug)]
pub enum WalkDirError {
	#[error("error while operating on git backend: {0}")]	
	StorageBackendError(#[from] StorageBackendError),
	#[error("error while operating on git backend: {0}")]	
	DataLoaderError(#[from] DataLoaderError),
}

#[derive(Error, Debug)]
pub enum FFIError {
	#[error("error convert string between cffi: {0}")]
	StringConvertNulError(#[from] NulError),
	#[error("error convert string between cffi: {0}")]
	StringConvertUtf8Error(#[from] Utf8Error),
	#[error("error building backend from config: {0}")]	
	CreateBackendError(&'static str),
	#[error("error building backend from config: {0}")]	
	StorageBackendError(#[from] StorageBackendError),

}




macro_rules! print_error_with_switch {
	($($arg:tt)*) => ({
		unsafe{
			if crate::PRINT_BACKGROUND_WATCHER_ERROR {
				eprintln!($($arg)*);
			}
		}
    })
}