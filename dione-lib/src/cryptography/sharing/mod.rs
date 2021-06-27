use alloc::vec::Vec;

pub mod shamir;

#[derive(Debug, PartialEq)]
pub enum SharingError {
	WrongThresholdAndNumber(u8, u8),
}

pub trait SharingAlgorithm {
	fn share(&self, data: &[u8], n: u8, t: u8) -> Result<Vec<Vec<u8>>, SharingError>;
	fn reconstruct(&self, inp: &[Vec<u8>]) -> Result<Vec<u8>, SharingError>;
}
