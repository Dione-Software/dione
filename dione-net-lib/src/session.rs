use dione_lib::cryptography::ratchet::{MagicRatchet, AddressShare};
use crate::bundle::AliceBob;
use serde::{Serialize, Deserialize};
use std::borrow::Borrow;

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

#[derive(Serialize, Deserialize)]
struct ExSession {
	partner: AliceBob,
	#[serde(with = "serde_bytes")]
	magic_ratchet: Vec<u8>
}

impl From<&Session> for ExSession {
	fn from(session: &Session) -> Self {
		Self {
			partner: session.partner,
			magic_ratchet: session.magic_ratchet.export()
		}
	}
}

impl From<&ExSession> for Session {
	fn from(ex_session: &ExSession) -> Self {
		Self {
			partner: ex_session.partner,
			magic_ratchet: MagicRatchet::import(&ex_session.magic_ratchet),
		}
	}
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
		self.magic_ratchet.next_addresses();
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

	pub fn export(&self) -> Vec<u8> {
		let ex: ExSession = self.into();
		bincode::serialize(&ex).unwrap()
	}

	pub fn import(inp: &[u8]) -> Self {
		let ex: ExSession = bincode::deserialize(inp).unwrap();
		ex.borrow().into()
	}
}
