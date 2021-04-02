pub mod fates;
pub use fates::Fate;
pub use fates_macro::fate;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
