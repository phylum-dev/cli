pub mod api;
pub mod config;
pub mod types;
pub mod restson;

#[macro_use]
extern crate log;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

}
