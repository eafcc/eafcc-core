mod rule_engine;
mod model;
mod error;
mod parser;
mod storage_backends;
mod cfg_center;
// mod ffi;
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
