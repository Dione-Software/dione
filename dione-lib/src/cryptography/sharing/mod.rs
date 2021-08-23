use alloc::vec::Vec;

pub mod shamir;
pub mod block;

#[derive(Debug, PartialEq)]
pub enum SharingError {
	WrongThresholdAndNumber(u8, u8), // The wrong threshold this is just a test commit, trying to fix my github
}

pub trait ThresholdSharingAlgorithm {
	fn share(&self, data: &[u8], n: u8, t: u8) -> Result<Vec<Vec<u8>>, SharingError>;
	fn reconstruct(&self, inp: &[Vec<u8>]) -> Result<Vec<u8>, SharingError>;
}

pub trait SharingAlgorithm {
	fn share(&self, data: &[u8], n: usize) -> Result<Vec<Vec<u8>>, SharingError>;
	fn reconstruct(&self, inp: &[Vec<u8>]) -> Result<Vec<u8>, SharingError>;
}