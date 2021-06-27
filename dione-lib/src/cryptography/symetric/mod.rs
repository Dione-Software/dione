use alloc::vec::Vec;

pub mod aes_aead;


#[derive(Debug, PartialEq)]
pub enum AeadError {
	DecryptionFailure,
	InvalidKeyLength(usize),
	InvalidNonceLength(usize),
}

pub trait AeadCipher {
	fn encrypt(plaintext: &[u8], key: &[u8], nonce: &[u8]) -> Result<Vec<u8>, AeadError>;
	fn decrypt(ciphertext: &[u8], key: &[u8], nonce: &[u8]) -> Result<Vec<u8>, AeadError>;
}