use crate::cryptography::symetric::dh::DhKeyPair;
use p256::{PublicKey, SecretKey};
use hashbrown::HashMap;
use alloc::vec::Vec;
use zeroize::Zeroize;
use rand_core::OsRng;
use crate::cryptography::ratchet::kdf_root::kdf_rk;
use crate::cryptography::ratchet::kdf_chain::kdf_ck;

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

	pub fn get_address_send(&mut self) -> [u8; 32] {
		let (cks, mk) = kdf_ck(&self.cks.unwrap());
		self.cks = Some(cks);
		self.ns += 1;
		mk
	}

	pub fn get_address_recv(&mut self) -> [u8; 32] {
		todo!()
	}
}