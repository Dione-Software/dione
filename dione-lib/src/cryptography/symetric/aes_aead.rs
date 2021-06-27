use crate::cryptography::symetric::{AeadCipher, AeadError};
use alloc::vec::Vec;
use aes_gcm::{Nonce, Aes256Gcm};
use aes_gcm::aead::{Aead, NewAead};

pub struct AesGcm;

impl AeadCipher for AesGcm {
	fn encrypt(plaintext: &[u8], key: &[u8], nonce: &[u8]) -> Result<Vec<u8>, AeadError> {
		if key.len() != 32 {
			return Err(AeadError::InvalidKeyLength(key.len()))
		}
		if nonce.len() != 12 {
			return Err(AeadError::InvalidNonceLength(nonce.len()))
		}
		let nonce = Nonce::from_slice(nonce);
		let key = aes_gcm::Key::from_slice(key);
		let cipher = Aes256Gcm::new(key);
		let ciphertext = cipher.encrypt(nonce, plaintext).unwrap();
		Ok(ciphertext)
	}

	fn decrypt(ciphertext: &[u8], key: &[u8], nonce: &[u8]) -> Result<Vec<u8>, AeadError> {
		let  key = aes_gcm::Key::from_slice(key);
		let cipher = Aes256Gcm::new(key);
		let nonce = Nonce::from_slice(nonce);
		match cipher.decrypt(nonce, ciphertext) {
			Ok(d) => Ok(d),
			Err(_) => Err(AeadError::DecryptionFailure)
		}
	}
}

#[cfg(test)]
mod aes_gcm_test {
	use crate::cryptography::symetric::aes_aead::AesGcm;
	use crate::cryptography::symetric::{AeadCipher, AeadError};

	#[test]
	fn basic_encrypt_decrypt() {
		let key = b"an example very very secret key.";
		let nonce = b"unique nonce";
		let plaintext = b"This is just the fucking cleartext.";
		let ciphertext = AesGcm::encrypt(plaintext, key, nonce).unwrap();
		let decrypted = AesGcm::decrypt(&ciphertext, key, nonce).unwrap();
		let plaintext_vec = plaintext.to_vec();
		assert_eq!(plaintext_vec.len(), decrypted.len())
	}

	#[test]
	fn invalid_key_length() {
		let key = b"an example very very secret key";
		let nonce = b"unique nonce";
		let plaintext = b"This is just the fucking cleartext.";
		let encrypted_err = AesGcm::encrypt(plaintext, key, nonce).unwrap_err();
		assert_eq!(encrypted_err, AeadError::InvalidKeyLength(31))
	}

	#[test]
	fn invalid_nonce_length() {
		let key = b"an example very very secret key.";
		let nonce = b"unique nonc"; // lol a spelling error in the source code. That probably never happened.
		let plaintext = b"This is just the fucking cleartext."; // Swearing in source code. It gets even better.
		let encrypted_err = AesGcm::encrypt(plaintext, key, nonce).unwrap_err();
		assert_eq!(encrypted_err, AeadError::InvalidNonceLength(11))
	}

	#[test]
	fn false_encryption() {
		let key = b"An example very very secret key.";
		let dif_key = b"An exumple very very secret key.";
		let nonce = b"unique nonce";
		let plaintext = b"This is just the fucking cleartext.";
		let encrypted = AesGcm::encrypt(plaintext, key, nonce).unwrap();
		let decrypted = AesGcm::decrypt(&encrypted, dif_key, nonce).unwrap_err();
		assert_eq!(decrypted, AeadError::DecryptionFailure)
	}
}