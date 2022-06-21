// #[macro_use]
extern crate actix_web;


mod utils;
pub use utils::*;

mod middleware;
pub use middleware::*;

mod chimes;
pub use chimes::*;


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
