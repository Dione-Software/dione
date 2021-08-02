use double_ratchet_2::ratchet::RatchetEncHeader;
use alloc::vec::Vec;
use crate::cryptography::ratchet::address_ratchet::{AddressHeader, AddressRatchet};
use serde::{Serialize, Deserialize};
use alloc::borrow::ToOwned;
use core::borrow::Borrow;
use crate::cryptography::sharing::shamir::ShamirSecretSharing;
use crate::cryptography::sharing::{ThresholdSharingAlgorithm, SharingAlgorithm};
use p256::PublicKey;
use crate::cryptography::sharing::block::BlockSharing;
use hashbrown::HashSet;

mod header;
mod kdf_chain;
mod kdf_root;
mod address_ratchet;

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
			message: d.message.clone(),
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

pub struct MagicRatchet {
	enc_ratchet: RatchetEncHeader,
	share_number: usize, // Number of shares to produce
	address_ratchets: Vec<AddressRatchet>,
	skipped_addresses: HashSet<Vec<[u8; 32]>>,
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

	pub fn send(&mut self, data: Vec<u8>, ad: &[u8]) -> Result<Vec<AddressShare>, &'static str> {
		let addresses: Vec<[u8; 32]> = self.address_ratchets.iter_mut().map(|e| e.ratchet_send().1).collect();
		let address_header = self.address_ratchets.iter().map(|e| e.give_header()).collect();
		let decrypted_message = DecryptedMessage::new(address_header, data);
		let message_bytes: Vec<u8> = decrypted_message.borrow().into();
		let encrypted = self.enc_ratchet.ratchet_encrypt(&message_bytes, ad);
		let shared_header = SharedHeader::from(&encrypted);
		let shared_header_bytes = bincode::serialize(&shared_header).unwrap();
		let shamir = ShamirSecretSharing::default();
		let block = crate::cryptography::sharing::block::BlockSharing::default();
		let shares_shared_header = shamir.share(&shared_header_bytes, self.share_number as u8, self.share_number as u8).unwrap();
		let shares_content = block.share(&encrypted.1, self.share_number).unwrap();
		Ok(shares_content.iter().zip(shares_shared_header.iter())
			.map(|e| Share::new(e.1.to_vec(), e.0.to_vec()))
			.map(|e| bincode::serialize(&e).unwrap())
			.zip(addresses.iter())
			.map(|e| (e.1.to_owned(), e.0))
			.collect())
	}

	pub fn recv(&mut self, data: &Vec<AddressShare>, ad: &[u8]) -> Result<Vec<u8>, &'static str> {
		let d: Vec<([u8; 32], Share)> = data
			.iter()
			.map(|e| (e.0, bincode::deserialize(&e.1).unwrap()))
			.collect();


		let header_shares: Vec<Vec<u8>> = d.iter().map(|e| e.1.header.clone()).collect();
		let shamir = ShamirSecretSharing::default();
		let block = BlockSharing::default();
		let shared_header_bytes = shamir.reconstruct(&header_shares).unwrap();
		let shared_header: SharedHeader = bincode::deserialize(&shared_header_bytes).unwrap();

		let shares_content: Vec<Vec<u8>> = d.iter().map(|e| e.1.content.clone()).collect();
		let encrypted_content = block.reconstruct(&shares_content).unwrap();

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
		let encrypted = magic_ratchet_alice.send(data.clone(), b"").unwrap();
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
		let encrypted = magic_ratchet_alice.send(b"".to_vec(), b"").unwrap();
		let decrypted = magic_ratchet_bob.recv(&encrypted, b"").unwrap();
		assert_eq!(b"".to_vec(), decrypted);
		let send_addresses: Vec<[u8; 32]> = magic_ratchet_alice.send(b"".to_vec(), b"").unwrap().iter().map(|e| e.0).collect();
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
		let encrypted = magic_ratchet_alice.send(b"".to_vec(), b"").unwrap();
		let decrypted = magic_ratchet_bob.recv(&encrypted, b"").unwrap();
		assert_eq!(b"".to_vec(), decrypted);
		let data = b"This is data".to_vec();
		let encrypted = magic_ratchet_bob.send(data.clone(), b"ad").unwrap();
		let decrypted = magic_ratchet_alice.recv(&encrypted, b"ad").unwrap();
		assert_eq!(decrypted, data)
	}
}