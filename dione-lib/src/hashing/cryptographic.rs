use alloc::vec::Vec;
use digest::Digest;

pub fn sha512_hash_bytes(data: &[u8]) -> Vec<u8> {
	let mut hasher = ring_compat::digest::Sha512::new();
	hasher.update(data);
	Digest::finalize_reset(&mut hasher).to_vec()
}

#[cfg(test)]
mod sha512_test {
	use crate::hashing::cryptographic::sha512_hash_bytes;

	#[test]
	fn sha512_hash_test_eq() {
		let data1 = b"Hello World";
		let data2 = b"Hello World";
		let number1 = sha512_hash_bytes(data1);
		let number2 = sha512_hash_bytes(data2);
		assert_eq!(number1, number2)
	}

	#[test]
	fn sha512_hash_test_nq() {
		let data1 = b"Hello World";
		let data2 = b"Hallo World2";
		let number1 = sha512_hash_bytes(data1);
		let number2 = sha512_hash_bytes(data2);
		assert_ne!(number1, number2)
	}
}