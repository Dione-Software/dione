use alloc::prelude::v1::Vec;
use alloc::string::ToString;
use core::convert::TryFrom;

use rand_core::OsRng;
use sharks::{Share, Sharks};

use crate::cryptography::sharing::{SharingAlgorithm, SharingError};

pub struct ShamirSecretSharingShark;

impl SharingAlgorithm for ShamirSecretSharingShark {
	fn share(&self, data: &[u8], n: usize, t: u8) -> Vec<Vec<u8>> {
		let sharks = Sharks(t);
		let dealer = sharks.dealer_rng(data, &mut OsRng);
		let shares: Vec<Share> = dealer.take(n).collect();
		shares.iter().map(|e| Vec::from(e)).collect()
	}

	fn reconstruct(&self, inp: &[Vec<u8>], t: u8) -> Result<Vec<u8>, SharingError> {
		let sharks = Sharks(t);
		let shares: Vec<Share> = inp
			.iter()
			.map(|e| Share::try_from(e.as_slice()).unwrap())
			.collect();
		let secret = sharks.recover(&shares);
		match secret {
			Ok(d) => Ok(d),
			Err(e) => Err(SharingError::ThresholdNotMet(e.to_string())),
		}
	}
}

pub struct ThresholdSecretSharing;

impl SharingAlgorithm for ThresholdSecretSharing {
	fn share(&self, _data: &[u8], _n: usize, _t: u8) -> Vec<Vec<u8>> {
		todo!()
	}

	fn reconstruct(&self, _inp: &[Vec<u8>], _t: u8) -> Result<Vec<u8>, SharingError> {
		todo!()
	}
}


#[test]
fn sharing_reconstruction() {
	let data = b"Helo World This Is Just A Test";
	let sharer = ShamirSecretSharingShark;
	let shares = sharer.share(data, 200, 64);
	let reconstructed = sharer.reconstruct(&shares, 64).unwrap();
	assert_eq!(data.to_vec(), reconstructed)
}
