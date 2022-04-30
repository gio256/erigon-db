#![allow(unused_imports)]
#![allow(unused)]
mod erigon;
pub mod kv;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
