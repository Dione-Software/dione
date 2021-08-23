use rand_core::OsRng;
use core::fmt::{Debug, Formatter};
use core::fmt;
use p256::PublicKey as PublicKey;
use p256::ecdh::SharedSecret;
use p256::SecretKey;
use alloc::vec::Vec;
use alloc::string::ToString;
use p256::elliptic_curve::ecdh::diffie_hellman;

use zeroize::Zeroize;

#[derive(Clone)]
pub struct DhKeyPair {
	pub private_key: SecretKey,
	pub public_key: PublicKey,
}

impl Drop for DhKeyPair {
	fn drop(&mut self) {
		self.private_key = SecretKey::random(&mut OsRng);
		self.public_key = self.private_key.public_key();
	}
}

impl Zeroize for DhKeyPair {
	fn zeroize(&mut self) {
		self.private_key = SecretKey::random(&mut OsRng);
		self.public_key = self.private_key.public_key();
	}
}

impl DhKeyPair {
	fn ex_public_key_bytes(&self) -> Vec<u8> {
		self.public_key.to_string().as_bytes().to_vec()
	}
}

impl PartialEq for DhKeyPair {
	fn eq(&self, other: &Self) -> bool {
		if self.private_key.to_bytes() != other.private_key.to_bytes() {
			return false
		}
		if self.ex_public_key_bytes() != other.ex_public_key_bytes() {
			return false
		}
		true
	}
}

impl Debug for DhKeyPair {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("DhKeyPair")
			.field("private_key", &self.private_key.to_bytes())
			.field("public_key", &self.ex_public_key_bytes())
			.finish()
	}
}

impl Default for DhKeyPair {
	fn default() -> Self {
		Self::new()
	}
}

impl DhKeyPair {
	pub fn new() -> Self {
		let secret = SecretKey::random(&mut OsRng);
		let public = secret.public_key();
		DhKeyPair {
			private_key: secret,
			public_key: public,
		}
	}

	pub fn key_agreement(&self, public_key: &PublicKey) -> SharedSecret {
		diffie_hellman(self.private_key.to_secret_scalar(), public_key.as_affine())
	}
}

#[cfg(test)]
pub fn gen_shared_secret() -> SharedSecret {
	let alice_pair = DhKeyPair::new();
	let bob_pair = DhKeyPair::new();
	alice_pair.key_agreement(&bob_pair.public_key)
}

#[cfg(test)]
pub fn gen_key_pair() -> DhKeyPair {
	DhKeyPair::new()
}

#[cfg(test)]
mod tests {
	use crate::cryptography::symetric::dh::DhKeyPair;
	use alloc::string::ToString;

	#[test]
	fn key_generation() {
		let pair_1 = DhKeyPair::new();
		let pair_2 = DhKeyPair::new();
		assert_ne!(pair_1, pair_2)
	}

	#[test]
	fn key_agreement() {
		let alice_pair = DhKeyPair::new();
		let bob_pair = DhKeyPair::new();
		let alice_shared_secret = alice_pair.key_agreement(&bob_pair.public_key);
		let bob_shared_secret = bob_pair.key_agreement(&alice_pair.public_key);
		assert_eq!(alice_shared_secret.as_bytes(), bob_shared_secret.as_bytes())
	}

	#[test]
	fn ex_public_key() {
		let key_pair = DhKeyPair::new();
		let public_key_bytes = key_pair.ex_public_key_bytes();
		let extracted_pk = key_pair.public_key.to_string().as_bytes().to_vec();
		assert_eq!(extracted_pk, public_key_bytes)
	}

	#[test]
	fn nq_key_pair() {
		let key_pair1 = DhKeyPair::new();
		let mut key_pair2 = DhKeyPair::new();
		key_pair2.private_key = key_pair1.private_key.clone();
		assert_ne!(key_pair1, key_pair2)
	}

	#[test]
	fn eq_key_pair() {
		let key_pair1 = DhKeyPair::new();
		let key_pair2 = key_pair1.clone();
		assert_eq!(key_pair1, key_pair2)
	}

	#[test]
	fn test_format() {
		let key_pair = DhKeyPair::default();
		let _str = alloc::format!("{:?}", key_pair);
	}
}