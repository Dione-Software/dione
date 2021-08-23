use double_ratchet_2::ratchet::RatchetEncHeader;
use alloc::vec::Vec;
use crate::cryptography::ratchet::address_ratchet::{AddressHeader, AddressRatchet};
use serde::{Serialize, Deserialize};
use alloc::borrow::ToOwned;
use core::borrow::Borrow;
use crate::cryptography::sharing::shamir::ShamirSecretSharing;
use crate::cryptography::sharing::{ThresholdSharingAlgorithm, SharingAlgorithm, SharingError};
use p256::PublicKey;
use crate::cryptography::sharing::block::BlockSharing;
use hashbrown::HashSet;

mod header;
mod kdf_chain;
mod kdf_root;
mod address_ratchet;

#[derive(Debug)]
pub enum MagicRatchetError {
	SerializationError,
	DeserializationError,
	ShamirSharingError(SharingError),
	BlockSharingError(SharingError),
	ShamirReconstructError(SharingError),
	BlockReconstructError(SharingError),
}

struct DecryptedMessage {
	pub address_headers: Vec<AddressHeader>,
	message: Vec<u8>,
}

impl DecryptedMessage {
	pub(crate) fn new(address_headers: Vec<AddressHeader>, message: Vec<u8>) -> Self {
		Self {
			address_headers,
			message,
		}
	}
}

impl From<&[u8]> for DecryptedMessage {
	fn from(d: &[u8]) -> Self {
		let ex: ExDecryptedMessage = bincode::deserialize(d).unwrap();
		Self::from(ex)
	}
}

impl From<&DecryptedMessage> for Vec<u8> {
	fn from(d: &DecryptedMessage) -> Self {
		let ex = ExDecryptedMessage::from(d);
		bincode::serialize(&ex).unwrap()
	}
}

#[derive(Deserialize, Serialize)]
struct NestedVec {
	#[serde(with = "serde_bytes")]
	field: Vec<u8>
}

impl From<Vec<u8>> for NestedVec {
	fn from(d: Vec<u8>) -> Self {
		Self {
			field: d
		}
	}
}

#[derive(Deserialize, Serialize)]
struct ExDecryptedMessage {
	address_headers: Vec<NestedVec>,
	#[serde(with = "serde_bytes")]
	message: Vec<u8>,
}

impl From<&DecryptedMessage> for ExDecryptedMessage {
	fn from(d: &DecryptedMessage) -> Self {
		let headers_bytes = d.address_headers.iter()
			.map(|e| NestedVec::from(Vec::from(e.to_owned())))
			.collect();
		Self {
			address_headers: headers_bytes,
			message: d.message.clone(),
		}
	}
}

impl From<ExDecryptedMessage> for DecryptedMessage {
	fn from(d: ExDecryptedMessage) -> Self {
		let headers = d.address_headers
			.iter()
			.map(|e| AddressHeader::from(e.field.as_slice()))
			.collect();
		Self {
			address_headers: headers,
			message: d.message,
		}
	}
}

type AddressShare = ([u8; 32], Vec<u8>);

type EncryptedMessageWithExtra = ((Vec<u8>, [u8; 12]), Vec<u8>, [u8; 12]);

#[derive(Serialize, Deserialize)]
struct SharedHeader {
	#[serde(with = "serde_bytes")]
	header: Vec<u8>,
	header_nonce: [u8; 12],
	encrypted_nonce: [u8; 12],
}

impl From<&EncryptedMessageWithExtra> for SharedHeader {
	fn from(d: &EncryptedMessageWithExtra) -> Self {
		Self {
			header: d.0.0.clone(),
			header_nonce: d.0.1,
			encrypted_nonce: d.2,
		}
	}
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Share {
	#[serde(with = "serde_bytes")]
	header: Vec<u8>,
	#[serde(with = "serde_bytes")]
	content: Vec<u8>,
}

impl Share {
	pub(crate) fn new(header: Vec<u8>, content: Vec<u8>) -> Self {
		Self {
			header,
			content
		}
	}
}

#[derive(PartialEq, Debug)]
pub struct MagicRatchet {
	enc_ratchet: RatchetEncHeader,
	share_number: usize, // Number of shares to produce
	address_ratchets: Vec<AddressRatchet>,
	skipped_addresses: HashSet<Vec<[u8; 32]>>,
}

#[derive(Serialize, Deserialize)]
struct ExMagicRatchet {
	#[serde(with = "serde_bytes")]
	enc_ratchet: Vec<u8>,
	share_number: usize,
	address_ratchets: Vec<NestedVec>,
	skipped_addresses: HashSet<Vec<[u8; 32]>>,
}

impl From<&MagicRatchet> for ExMagicRatchet {
	fn from(mr: &MagicRatchet) -> Self {
		let enc_ratchet = mr.enc_ratchet.export();
		let share_number = mr.share_number;
		let address_ratchets = mr.address_ratchets
			.iter()
			.map(|e| e.export())
			.map(|e| NestedVec::from(e))
			.collect();
		let skipped_addresses = mr.skipped_addresses.clone();
		Self {
			enc_ratchet,
			share_number,
			address_ratchets,
			skipped_addresses
		}
	}
}

impl From<&ExMagicRatchet> for MagicRatchet {
	fn from(ex_mr: &ExMagicRatchet) -> Self {
		let enc_ratchet = RatchetEncHeader::import(&ex_mr.enc_ratchet);
		let share_number = ex_mr.share_number;
		let address_ratchets = ex_mr.address_ratchets
			.iter()
			.map(|e| e.field.clone())
			.map(|e| AddressRatchet::import(&e))
			.collect();
		let skipped_addresses = ex_mr.skipped_addresses.clone();
		Self {
			enc_ratchet,
			share_number,
			address_ratchets,
			skipped_addresses
		}
	}
}

impl MagicRatchet {
	pub fn init_alice(enc_rk: [u8; 32], enc_pk: PublicKey, shka: [u8; 32], snhkb: [u8; 32], share_number: usize, address_rks: Vec<[u8; 32]>, address_pks: Vec<PublicKey>) -> Self {
		let enc_ratchet = RatchetEncHeader::init_alice(enc_rk, enc_pk, shka, snhkb);
		let address_ratchets = address_rks.iter().zip(address_pks.iter())
			.map(|e| AddressRatchet::init_alice(e.0.to_owned(), e.1.to_owned()))
			.collect();
		Self {
			enc_ratchet,
			share_number,
			address_ratchets,
			skipped_addresses: HashSet::new()
		}
	}

	pub fn init_bob(enc_rk: [u8; 32], shka: [u8; 32], snhkb: [u8; 32], share_number: usize, address_rks: Vec<[u8; 32]>)  -> (Self, PublicKey, Vec<PublicKey>) {
		let (enc_ratchet, enc_pk) = RatchetEncHeader::init_bob(enc_rk, shka, snhkb);
		let (address_ratchets, address_pks): (Vec<_>, Vec<_>) = address_rks
			.iter()
			.map(|e| AddressRatchet::init_bob(*e))
			.unzip();
		(
			Self {
				enc_ratchet,
				share_number,
				address_ratchets,
				skipped_addresses: HashSet::new()
			}, enc_pk, address_pks
		)
	}

	pub fn send(&mut self, data: &[u8], ad: &[u8]) -> Result<Vec<AddressShare>, MagicRatchetError> {
		let (address_header, addresses): (Vec<AddressHeader>, Vec<[u8; 32]>) = self.address_ratchets.iter_mut().map(|e| e.ratchet_send().unwrap()).unzip();
		let decrypted_message = DecryptedMessage::new(address_header, data.to_vec());
		let message_bytes: Vec<u8> = decrypted_message.borrow().into();
		let encrypted = self.enc_ratchet.ratchet_encrypt(&message_bytes, ad);
		let shared_header = SharedHeader::from(&encrypted);
		let shared_header_bytes = match bincode::serialize(&shared_header) {
			Ok(d) => d,
			Err(_) => {
				return Err(MagicRatchetError::SerializationError)
			}
		};
		let shamir = ShamirSecretSharing::default();
		let block = crate::cryptography::sharing::block::BlockSharing::default();
		let shares_shared_header = match shamir.share(&shared_header_bytes, self.share_number as u8, self.share_number as u8) {
			Ok(d) => d,
			Err(e) => { return Err(MagicRatchetError::ShamirSharingError(e)) }
		};
		let shares_content = match block.share(&encrypted.1, self.share_number) {
			Ok(d) => d,
			Err(e) => { return Err(MagicRatchetError::BlockSharingError(e)) }
		};
		Ok(shares_content.iter().zip(shares_shared_header.iter())
			.map(|e| Share::new(e.1.to_vec(), e.0.to_vec()))
			.map(|e| bincode::serialize(&e).unwrap())
			.zip(addresses.iter())
			.map(|e| (e.1.to_owned(), e.0))
			.collect())
	}

	pub fn recv(&mut self, data: &[AddressShare], ad: &[u8]) -> Result<Vec<u8>, MagicRatchetError> {
		let d: Vec<([u8; 32], Share)> = data
			.iter()
			.map(|e| (e.0, bincode::deserialize(&e.1).unwrap()))
			.collect();


		let header_shares: Vec<Vec<u8>> = d.iter().map(|e| e.1.header.clone()).collect();
		let shamir = ShamirSecretSharing::default();
		let block = BlockSharing::default();
		let shared_header_bytes = match shamir.reconstruct(&header_shares) {
			Ok(d) => d,
			Err(e) => {
				return Err(MagicRatchetError::ShamirReconstructError(e))
			}
		};
		let shared_header: SharedHeader = match bincode::deserialize(&shared_header_bytes) {
			Ok(d) => d,
			Err(_) => {
				return Err(MagicRatchetError::DeserializationError)
			}
		};

		let shares_content: Vec<Vec<u8>> = d.iter().map(|e| e.1.content.clone()).collect();
		let encrypted_content = match block.reconstruct(&shares_content) {
			Ok(d) => d,
			Err(e) => {
				return Err(MagicRatchetError::BlockReconstructError(e))
			}
		};

		let (decrypted, _header) = self.enc_ratchet.ratchet_decrypt_w_header(&(shared_header.header, shared_header.header_nonce),
		&encrypted_content,
			&shared_header.encrypted_nonce,
			ad
		);
		let decrypted_message = DecryptedMessage::from(decrypted.as_slice());

		self.address_ratchets.iter_mut()
			.zip(decrypted_message.address_headers.iter())
			.for_each(|ratchet_and_header| {
				ratchet_and_header.0.proccess_recv(ratchet_and_header.1);
			});

		Ok(decrypted_message.message)
	}

	pub fn next_addresses(&mut self) -> Vec<[u8; 32]> {
		let addresses: Vec<[u8; 32]> = self.address_ratchets.iter_mut()
			.map(|e| e.next_address().unwrap()).collect();
		self.skipped_addresses.insert(addresses.clone());
		addresses
	}

	pub fn export(&self) -> Vec<u8> {
		let ex: ExMagicRatchet = self.into();
		bincode::serialize(&ex).unwrap()
	}

	pub fn import(inp: &[u8]) -> Self {
		let ex: ExMagicRatchet = bincode::deserialize(inp).unwrap();
		MagicRatchet::from(&ex)
	}
}

impl Iterator for MagicRatchet {
	type Item = Vec<[u8; 32]>;

	fn next(&mut self) -> Option<Self::Item> {
		Some(self.next_addresses())
	}
}

#[cfg(test)]
mod magic_ratchet_test {
	use crate::cryptography::ratchet::MagicRatchet;
	use alloc::vec::Vec;

	#[test]
	fn basic_encrypt_decrypt() {
		let data = b"Hello World".to_vec();
		let enc_rk = [0; 32];
		let shka = [1; 32];
		let snhkb = [2; 32];
		let address_rks = alloc::vec![[3; 32], [4; 32], [5; 32]];
		let number_shares = 3;
		let (mut magic_ratchet_bob, enc_pk, address_pks) = MagicRatchet::init_bob(enc_rk, shka, snhkb, number_shares, address_rks.clone());
		let mut magic_ratchet_alice = MagicRatchet::init_alice(enc_rk, enc_pk, shka, snhkb, number_shares, address_rks, address_pks);
		let encrypted = magic_ratchet_alice.send(&data, b"").unwrap();
		let decrypted = magic_ratchet_bob.recv(&encrypted, b"").unwrap();
		assert_eq!(data, decrypted)
	}

	#[test]
	fn address_test() {
		let enc_rk = [0; 32];
		let shka = [1; 32];
		let snhkb = [2; 32];
		let address_rks = alloc::vec![[3; 32], [4; 32], [5; 32]];
		let number_shares = 3;
		let (mut magic_ratchet_bob, enc_pk, address_pks) = MagicRatchet::init_bob(enc_rk, shka, snhkb, number_shares, address_rks.clone());
		let mut magic_ratchet_alice = MagicRatchet::init_alice(enc_rk, enc_pk, shka, snhkb, number_shares, address_rks, address_pks);
		let encrypted = magic_ratchet_alice.send(b"", b"").unwrap();
		let decrypted = magic_ratchet_bob.recv(&encrypted, b"").unwrap();
		assert_eq!(b"".to_vec(), decrypted);
		let send_addresses: Vec<[u8; 32]> = magic_ratchet_alice.send(b"", b"").unwrap().iter().map(|e| e.0).collect();
		magic_ratchet_bob.next_addresses();
		let recv_addresses = magic_ratchet_bob.next_addresses();
		assert_eq!(send_addresses, recv_addresses);
	}

	#[test]
	fn alice_send() {
		let enc_rk = [0; 32];
		let shka = [1; 32];
		let snhkb = [2; 32];
		let address_rks = alloc::vec![[3; 32], [4; 32], [5; 32]];
		let number_shares = 3;
		let (mut magic_ratchet_bob, enc_pk, address_pks) = MagicRatchet::init_bob(enc_rk, shka, snhkb, number_shares, address_rks.clone());
		let mut magic_ratchet_alice = MagicRatchet::init_alice(enc_rk, enc_pk, shka, snhkb, number_shares, address_rks, address_pks);
		let encrypted = magic_ratchet_alice.send(b"", b"").unwrap();
		let decrypted = magic_ratchet_bob.recv(&encrypted, b"").unwrap();
		assert_eq!(b"".to_vec(), decrypted);
		let data = b"This is data".to_vec();
		let encrypted = magic_ratchet_bob.send(&data, b"ad").unwrap();
		let decrypted = magic_ratchet_alice.recv(&encrypted, b"ad").unwrap();
		assert_eq!(decrypted, data)
	}

	#[test]
	fn import_export() {
		let enc_rk = [0; 32];
		let shka = [1; 32];
		let snhkb = [2; 32];
		let address_rks = alloc::vec![[3; 32], [4; 32], [5; 32]];
		let number_shares = 3;
		let (magic_ratchet_bob, enc_pk, address_pks) = MagicRatchet::init_bob(enc_rk, shka, snhkb, number_shares, address_rks.clone());
		let magic_ratchet_alice = MagicRatchet::init_alice(enc_rk, enc_pk, shka, snhkb, number_shares, address_rks, address_pks);

		let ex_magic_ratchet_bob = magic_ratchet_bob.export();
		let im_magic_ratchet_bob = MagicRatchet::import(&ex_magic_ratchet_bob);

		assert_eq!(im_magic_ratchet_bob, magic_ratchet_bob);

		let ex_magic_ratchet_alice = magic_ratchet_alice.export();
		let im_magic_ratchet_alice = MagicRatchet::import(&ex_magic_ratchet_alice);

		assert_eq!(im_magic_ratchet_alice, magic_ratchet_alice);
	}
}