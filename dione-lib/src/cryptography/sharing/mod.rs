pub mod shamir;

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug)]
pub enum SharingError {
    ThresholdNotMet(String),
}

pub trait SharingAlgorithm {
    fn share(&self, data: &[u8], n: usize, t: u8) -> Vec<Vec<u8>>;
    fn reconstruct(&self, inp: &[Vec<u8>], t: u8) -> Result<Vec<u8>, SharingError>;
}
