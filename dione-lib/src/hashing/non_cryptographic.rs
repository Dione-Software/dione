use alloc::vec::Vec;

pub fn adler_hash_bytes(data: &[u8]) -> Vec<u8> {
	let mut hasher = adler::Adler32::new();
	hasher.write_slice(data);
	hasher.checksum().to_be_bytes().to_vec()
}

pub fn seahash_hash_bytes(data: &[u8]) -> Vec<u8> {
	seahash::hash(data).to_be_bytes().to_vec()
}

#[cfg(test)]
mod adler_test {
    use crate::hashing::non_cryptographic::adler_hash_bytes;

    #[test]
	fn adler_hash_eq() {
		let data1 = b"Hello World";
		let data2 = b"Hello World";
		let number1 = adler_hash_bytes(data1);
		let number2 = adler_hash_bytes(data2);
		assert_eq!(number1, number2)
	}

	#[test]
	fn adler_hash_nq() {
		let data1 = b"Hello World";
		let data2 = b"Hallo World";
		let number1 = adler_hash_bytes(data1);
		let number2 = adler_hash_bytes(data2);
		assert_ne!(number1, number2)
	}
}

#[cfg(test)]
mod seahash_test {
    use crate::hashing::non_cryptographic::seahash_hash_bytes;

    #[test]
	fn seahash_hash_eq() {
		let data1 = b"Hello World";
		let data2 = b"Hello World";
		let number1 = seahash_hash_bytes(data1);
		let number2 = seahash_hash_bytes(data2);
		assert_eq!(number1, number2)
	}

	#[test]
	fn seahash_hash_nq() {
		let data1 = b"Hello World";
		let data2 = b"Hallo World2";
		let number1 = seahash_hash_bytes(data1);
		let number2 = seahash_hash_bytes(data2);
		assert_ne!(number1, number2)
	}
}
