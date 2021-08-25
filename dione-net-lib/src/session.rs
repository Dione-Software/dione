use dione_lib::cryptography::ratchet::{MagicRatchet, AddressShare};
use crate::bundle::AliceBob;
use sled::Db;
use std::collections::HashMap;
use uuid::Uuid;

pub struct SessionBuilder {
	partner: Option<AliceBob>,
	magic_ratchet: Option<MagicRatchet>,
}

impl Default for SessionBuilder {
	fn default() -> Self {
		Self {
			partner: None,
			magic_ratchet: None
		}
	}
}

impl SessionBuilder {
	pub fn partner(mut self, partner: AliceBob) -> Self {
		self.partner = Some(partner);
		self
	}
	pub fn magic_ratchet(mut self, magic_ratchet: MagicRatchet) -> Self {
		self.magic_ratchet = Some(magic_ratchet);
		self
	}

	pub fn build(self) -> Session {
		Session::new(self.partner.unwrap(), self.magic_ratchet.unwrap())
	}
}

#[derive(PartialEq, Debug)]
pub struct Session {
	partner: AliceBob,
	magic_ratchet: MagicRatchet,
}


impl Session {
	pub fn new(partner: AliceBob, magic_ratchet: MagicRatchet) -> Self {
		Self {
			partner,
			magic_ratchet,
		}
	}

	pub fn make_init_message(&mut self) -> anyhow::Result<Vec<AddressShare>> {
		let res = self.magic_ratchet.send(b"", b"").unwrap();
		Ok(res)
	}
	
	pub fn process_init_message(&mut self, message: Vec<AddressShare>) {
		self.magic_ratchet.recv(&message, b"").unwrap();
	}

	pub fn send_message(&mut self, content: &[u8]) -> anyhow::Result<Vec<AddressShare>> {
		let res = self.magic_ratchet.send(content, b"").unwrap();
		Ok(res)
	}

	pub fn next_address(&mut self) -> anyhow::Result<Vec<[u8; 32]>> {
		let res = self.magic_ratchet.next_addresses();
		Ok(res)
	}

	pub fn recv_message(&mut self, data: &[AddressShare]) -> anyhow::Result<Vec<u8>> {
		let d = self.magic_ratchet.recv(data, b"").unwrap();
		Ok(d)
	}
}
