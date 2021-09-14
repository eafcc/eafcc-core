mod rule_engine;
mod model;
#[macro_use]
mod error;
mod parser;
mod storage_backends;
mod cfg_center;
mod ffi;

static mut PRINT_BACKGROUND_WATCHER_ERROR: bool = true; 

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
