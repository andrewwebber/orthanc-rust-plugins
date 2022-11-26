mod bindgen;
pub use self::bindgen::*;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}
