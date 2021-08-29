use crate::cryptography::symetric::dh::DhKeyPair;
use p256::{PublicKey, SecretKey};
use hashbrown::HashMap;
use alloc::vec::Vec;
use zeroize::Zeroize;
use rand_core::OsRng;
use crate::cryptography::ratchet::kdf_root::kdf_rk;
use crate::cryptography::ratchet::kdf_chain::kdf_ck;
use alloc::string::{ToString, String};
use serde::{Serialize, Deserialize};
use core::borrow::Borrow;

#[derive(Debug)]
pub enum AddressRatchetError {
	NoCks,
	SkippedTooManyKeys,
	NoCkr,
}

const MAX_SKIP: usize = 100;

pub type AddressHeader = crate::cryptography::ratchet::header::Header;

#[derive(PartialEq, Debug)]
pub struct AddressRatchet {
	dhs: DhKeyPair,
	dhr: Option<PublicKey>,
	rk: [u8; 32],
	ckr: Option<[u8; 32]>,
	cks: Option<[u8; 32]>,
	ns: usize,
	nr: usize,
	pn: usize,
	mkskipped: HashMap<(Vec<u8>, usize), [u8; 32]>
}

#[derive(Serialize, Deserialize)]
struct ExAddressRatchet {
	dhs: (String, String),
	dhr: Option<String>,
	rk: [u8; 32],
	ckr: Option<[u8; 32]>,
	cks: Option<[u8; 32]>,
	ns: usize,
	nr: usize,
	pn: usize,
	mkskipped: HashMap<(Vec<u8>, usize), [u8; 32]>
}

impl From<&AddressRatchet> for ExAddressRatchet {
	fn from(ar: &AddressRatchet) -> Self {
		let dhs_private = ar.dhs.private_key.to_jwk_string();
		let dhs_public = ar.dhs.public_key.to_jwk_string();
		let dhr = ar.dhr.map(|e| e.to_jwk_string());
		Self {
			dhs: (dhs_private, dhs_public),
			dhr,
			rk: ar.rk,
			ckr: ar.ckr,
			cks: ar.cks,
			ns: ar.ns,
			nr: ar.nr,
			pn: ar.pn,
			mkskipped: ar.mkskipped.clone(),
		}
	}
}

impl From<&ExAddressRatchet> for AddressRatchet {
	fn from(ex_ar: &ExAddressRatchet) -> Self {
		let dhs_private = SecretKey::from_jwk_str(&ex_ar.dhs.0).unwrap();
		let dhs_public = PublicKey::from_jwk_str(&ex_ar.dhs.1).unwrap();
		let dhs = DhKeyPair {
			private_key: dhs_private,
			public_key: dhs_public
		};
		let dhr = ex_ar.dhr.as_ref().map(|e| PublicKey::from_jwk_str(&e).unwrap());
		Self {
			dhs,
			dhr,
			rk: ex_ar.rk,
			ckr: ex_ar.ckr,
			cks: ex_ar.cks,
			ns: ex_ar.ns,
			nr: ex_ar.nr,
			pn: ex_ar.pn,
			mkskipped: ex_ar.mkskipped.clone(),
		}
	}
}

impl Drop for AddressRatchet {
	fn drop(&mut self) {
		self.dhs.zeroize();
		if let Some(mut _d) = self.dhr {
			let sk = SecretKey::random(&mut OsRng);
			_d = sk.public_key()
		}
		self.rk.zeroize();
		self.ckr.zeroize();
		self.cks.zeroize();
		self.ns.zeroize();
		self.nr.zeroize();
		self.pn.zeroize();
		self.mkskipped.clear();
	}
}

impl AddressRatchet {

	pub fn init_alice(sk: [u8; 32], bob_dh_public_key: PublicKey) -> Self {
		let dhs = DhKeyPair::new();
		let (rk, cks) = kdf_rk(&sk, &dhs.key_agreement(&bob_dh_public_key));

		Self {
			dhs,
			dhr: Some(bob_dh_public_key),
			rk,
			cks: Some(cks),
			ckr: None,
			ns: 0,
			nr: 0,
			pn: 0,
			mkskipped: HashMap::new(),
		}
	}

	pub fn init_bob(sk: [u8; 32]) -> (Self, PublicKey) {
		let dhs = DhKeyPair::new();
		let public_key = dhs.public_key;
		let ratchet = Self {
			dhs,
			dhr: None,
			rk: sk,
			cks: None,
			ckr: None,
			ns: 0,
			nr: 0,
			pn: 0,
			mkskipped: HashMap::new(),
		};
		(ratchet, public_key)
	}

	pub fn ratchet_send(&mut self) -> Result<(AddressHeader, [u8; 32]), AddressRatchetError> {
		let (cks, mk) = kdf_ck(&match self.cks {
			None => {
				return Err(AddressRatchetError::NoCks)
			}
			Some(d) => d,
		});
		self.cks = Some(cks);
		let header = AddressHeader::new(&self.dhs, self.pn, self.ns);
		self.ns += 1;
		Ok((header, mk))
	}

	pub fn try_skipped_message_keys(&mut self, header: &AddressHeader) -> bool {
		if self.mkskipped.contains_key(&(header.ex_public_key_bytes(), header.n)) {
			self.mkskipped.remove(&(header.ex_public_key_bytes(), header.n)).unwrap();
			true
		} else {
			false
		}
	}

	#[allow(dead_code)]
	pub fn skip_message_keys(&mut self, until: usize) -> Result<(), AddressRatchetError> {
		if self.nr + MAX_SKIP < until {
			return Err(AddressRatchetError::SkippedTooManyKeys);
		}
		match self.ckr {
			Some(d) => {
				while self.nr < until {
					let (ckr, mk) = kdf_ck(&d);
					self.ckr = Some(ckr);
					self.mkskipped.insert((self.dhr.unwrap().to_string().as_bytes().to_vec(), self.nr), mk);
					self.nr += 1
				}
				Ok(())
			},
			None => { Err(AddressRatchetError::NoCkr) }
		}
	}

	pub fn next_address(&mut self) -> Result<[u8; 32], AddressRatchetError> {
		if self.nr > MAX_SKIP {
			return Err(AddressRatchetError::SkippedTooManyKeys);
		}
		match self.ckr {
			Some(d) => {
				let (ckr, mk) = kdf_ck(&d);
				self.ckr = Some(ckr);
				self.mkskipped.insert((self.dhr.unwrap().to_string().as_bytes().to_vec(), self.nr), mk);
				self.nr += 1;
				Ok(mk)
			},
			None => { Err(AddressRatchetError::NoCkr) }
		}
	}

	pub fn proccess_recv(&mut self, header: &AddressHeader) {
		let _ = self.try_skipped_message_keys(header);
		if Some(header.public_key) != self.dhr {
			self.dhratchet(header);
		}
		if None == self.ckr {
			let (rk, ckr) = kdf_rk(&self.rk, &self.dhs.key_agreement(&self.dhr.unwrap()));
			self.rk = rk;
			self.ckr = Some(ckr);
		}
		// self.skip_message_keys(header.n).unwrap();
	}

	pub fn dhratchet(&mut self, header: &AddressHeader) {
		self.pn = self.ns;
		self.ns = 0;
		self.nr = 0;
		self.dhr = Some(header.public_key);
		let (rk, ckr) = kdf_rk(&self.rk, &self.dhs.key_agreement(&self.dhr.unwrap()));
		// let (rk, ckr) = kdf_ck(&self.rk);
		self.rk = rk;
		self.ckr = Some(ckr);
		// self.dhs = DhKeyPair::new();
		let (rk, cks) = kdf_rk(&self.rk, &self.dhs.key_agreement(&self.dhr.unwrap()));
		// let (rk, cks) = kdf_ck(&self.rk);
		self.rk = rk;
		self.cks = Some(cks);
	}

	pub fn export(&self) -> Vec<u8> {
		let ex = ExAddressRatchet::from(self);
		bincode::serialize(&ex).unwrap()
	}

	pub fn import(inp: &[u8]) -> Self {
		let ex: ExAddressRatchet = bincode::deserialize(inp).unwrap();
		ex.borrow().into()
	}
}