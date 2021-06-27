use alloc::vec::Vec;

use crate::cryptography::sharing::{SharingAlgorithm, SharingError};
use sss_rs::wrapped_sharing::{Secret, share};
use systemstat::System;

pub struct ShamirSecretSharing;

impl SharingAlgorithm for ShamirSecretSharing {
	fn share(&self, data: &[u8], n: u8, t: u8) -> Result<Vec<Vec<u8>>, SharingError> {
		let secret = Secret::InMemory(data.to_vec());
		if t > n {
			return Err(SharingError::WrongThresholdAndNumber(n, t))
		}
		Ok(share(secret, t, n, false).unwrap())
	}

	fn reconstruct(&self, inp: &[Vec<u8>]) -> Result<Vec<u8>, SharingError> {
		let mut secret = Secret::empty_in_memory();
		secret.reconstruct(inp.to_vec(), false).unwrap();
		let out_data = secret.unwrap_to_vec().unwrap();
		Ok(out_data)
	}
}

#[cfg(test)]
mod sss_test {
	use crate::cryptography::sharing::shamir::ShamirSecretSharing;
	use crate::cryptography::sharing::{SharingAlgorithm, SharingError};

	#[test]
	fn success_reconstruction() {
		let data = b"Hello World This Is Just A Test";
		let sharer = ShamirSecretSharing;
		let shares = sharer.share(data, 10, 5).unwrap();
		let recon = sharer.reconstruct(&shares).unwrap();
		assert_eq!(recon, data.to_vec())
	}

	#[test]
	fn basic_failure() {
		let data =  b"Hello World This Is Just A Test";
		let sharer = ShamirSecretSharing;
		let mut shares = sharer.share(data, 10, 10).unwrap();
		shares.pop().unwrap();
		let odata = sharer.reconstruct(&shares).unwrap();
		assert_ne!(data.to_vec(), odata)
	}

	#[test]
	fn threshold_panic() {
		let data = b"This will fail anyway";
		let sharer = ShamirSecretSharing;
		let shares = sharer.share(data, 5, 6);
		assert_eq!(shares.unwrap_err(), SharingError::WrongThresholdAndNumber(5, 6))
	}
}