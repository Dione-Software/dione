use dione_lib::cryptography::key_exchange::{AliceKeyBundle, IdentityKey, BobKeyBundle, AliceKeyBundleBytes, BobKeyBundleBytes};
use serde::{Serialize, Deserialize};

#[cfg(test)]
use dione_lib::cryptography::key_exchange::Key;

use thiserror::Error;
use dione_lib::cryptography::ratchet::{PublicKey, MagicRatchet};

#[cfg(test)]
use crate::session::SessionBuilder;

use std::borrow::Borrow;
use sled::Db;
use crate::HOST_BUNDLE_KEY;

#[derive(Debug, Error)]
pub enum BundleBuilderError {
	#[error("Partner not set (Alice or Bob)")]
	Partner,
	#[error("Identity key not set")]
	IdentityKey,
	#[error("Number of shares not set")]
	NumberShares,
}

#[derive(PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum AliceBob {
	Alice,
	Bob,
}

pub struct BundleBuilder {
	partner: Option<AliceBob>,
	number_shares: Option<usize>,
	identity_key: Option<IdentityKey>,
}

impl Default for BundleBuilder {
	fn default() -> Self {
		Self {
			partner: None,
			number_shares: None,
			identity_key: None,
		}
	}
}

impl BundleBuilder {
	pub fn partner(mut self, partner: AliceBob) -> Self {
		self.partner = Some(partner);
		self
	}

	pub fn number_shares(mut self, number_shares: usize) -> Self {
		self.number_shares = Some(number_shares);
		self
	}

	pub fn identity_key(mut self, identity_key: IdentityKey) -> Self {
		self.identity_key = Some(identity_key);
		self
	}

	pub fn build(self) -> anyhow::Result<PartnerBundle> {
		let partner = match self.partner {
			Some(d) => d,
			None => {
				return Err(anyhow::Error::from(BundleBuilderError::Partner))
			}
		};
		let identity_key = match self.identity_key {
			Some(d) => d,
			None => {
				return Err(anyhow::Error::from(BundleBuilderError::IdentityKey))
			}
		};
		let number_shares = match self.number_shares {
			Some(d) => d,
			None => {
				return Err(anyhow::Error::from(BundleBuilderError::NumberShares))
			}
		};
		match partner {
			AliceBob::Alice => {
				let enc_rk = AliceKeyBundle::new(&identity_key);
				let shka = AliceKeyBundle::new(&identity_key);
				let snhkb = AliceKeyBundle::new(&identity_key);
				let mut address_rks = Vec::with_capacity(number_shares);
				for _ in 0..number_shares {
					address_rks.push(AliceKeyBundle::new(&identity_key));
				}
				let alice_bundle =  AliceBundle {
					enc_rk,
					shka,
					snhkb,
					address_rks,
					number_shares
				};
				Ok(PartnerBundle::Alice(Box::new(alice_bundle)))
			}
			AliceBob::Bob => {
				let enc_rk = BobKeyBundle::new(&identity_key);
				let shka = BobKeyBundle::new(&identity_key);
				let snhkb = BobKeyBundle::new(&identity_key);
				let mut address_rks = Vec::with_capacity(number_shares);
				for _ in 0..number_shares {
					address_rks.push(BobKeyBundle::new(&identity_key));
				}
				let bob_bundle = BobBundle {
					enc_rk,
					enc_pk: None,
					shka,
					snhkb,
					address_rks,
					address_pks: None,
					number_shares
				};
				Ok(PartnerBundle::Bob(Box::new(bob_bundle)))
			}
		}
	}
}

#[derive(Debug, Error)]
pub enum PartnerBundleError {
	#[error("The number of shares is different `{0}` != `{1}`")]
	NumberOfSharesDifferent(usize, usize),
	#[error("Key Exchange error `{0}`")]
	KeyExchangeError(&'static str),
}

pub enum PartnerBundle {
	Alice(Box<AliceBundle>),
	Bob(Box<BobBundle>),
}

pub struct AliceBundle {
	enc_rk: AliceKeyBundle,
	shka: AliceKeyBundle,
	snhkb: AliceKeyBundle,
	address_rks: Vec<AliceKeyBundle>,
	number_shares: usize,
}

#[derive(Serialize, Deserialize)]
struct ExAliceBundle {
	enc_rk: AliceKeyBundleBytes,
	shka: AliceKeyBundleBytes,
	snhkb: AliceKeyBundleBytes,
	address_rks: Vec<AliceKeyBundleBytes>,
	number_shares: usize
}

impl From<&AliceBundle> for ExAliceBundle {
	fn from(ab: &AliceBundle) -> Self {
		let enc_rk = ab.enc_rk.borrow().into();
		let shka = ab.shka.borrow().into();
		let snhkb = ab.snhkb.borrow().into();
		let address_rks = ab.address_rks.iter()
			.map(|e| e.into())
			.collect();
		let number_shares = ab.number_shares;
		Self {
			enc_rk,
			shka,
			snhkb,
			address_rks,
			number_shares
		}
	}
}

impl From<&ExAliceBundle> for AliceBundle {
	fn from(ex_ab: &ExAliceBundle) -> Self {
		let enc_rk = ex_ab.enc_rk.borrow().into();
		let shka = ex_ab.shka.borrow().into();
		let snhkb = ex_ab.snhkb.borrow().into();
		let address_rks = ex_ab.address_rks.iter()
			.map(|e| e.into())
			.collect();
		let number_shares = ex_ab.number_shares;
		Self {
			enc_rk,
			shka,
			snhkb,
			address_rks,
			number_shares
		}
	}
}

impl AliceBundle {
	pub fn init(&self, bob_bundle: &BobBundle) -> anyhow::Result<(AliceBob, MagicRatchet)> {
		if self.number_shares != bob_bundle.number_shares {
			return Err(anyhow::Error::from(PartnerBundleError::NumberOfSharesDifferent(self.number_shares, bob_bundle.number_shares)))
		}
		let enc_rk = match self.enc_rk.key_exchange(&bob_bundle.enc_rk) {
			Ok(d) => d,
			Err(e) => {
				return Err(anyhow::Error::from(PartnerBundleError::KeyExchangeError(e)))
			}
		};
		let shka = match self.shka.key_exchange(&bob_bundle.shka) {
			Ok(d) => d,
			Err(e) => {
				return Err(anyhow::Error::from(PartnerBundleError::KeyExchangeError(e)))
			}
		};
		let snhkb = match self.snhkb.key_exchange(&bob_bundle.snhkb) {
			Ok(d) => d,
			Err(e) => {
				return Err(anyhow::Error::from(PartnerBundleError::KeyExchangeError(e)))
			}
		};
		let share_number = self.number_shares;
		let address_rks  = self.address_rks.iter().zip(bob_bundle.address_rks.iter())
			.map(|e| e.0.key_exchange(e.1).unwrap())
			.collect();
		let magic_ratchet = MagicRatchet::init_alice(
			enc_rk,
			bob_bundle.enc_pk.unwrap(),
			shka,
			snhkb,
			share_number,
			address_rks,
			bob_bundle.address_pks.clone().unwrap()
		);
		Ok((AliceBob::Alice, magic_ratchet))
	}

	pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
		let ex = ExAliceBundle::from(self);
		Ok(bincode::serialize(&ex)?)
	}

	pub fn from_bytes(inp: &[u8]) -> anyhow::Result<Self> {
		let ex: ExAliceBundle = bincode::deserialize(inp)?;
		Ok(ex.borrow().into())
	}

	pub fn strip(&self) -> Self {
		Self {
			enc_rk: self.enc_rk.strip(),
			shka: self.shka.strip(),
			snhkb: self.snhkb.strip(),
			address_rks: self.address_rks.iter().map(|e| e.strip()).collect(),
			number_shares: self.number_shares
		}
	}
}

pub struct BobBundle {
	enc_rk: BobKeyBundle,
	enc_pk: Option<PublicKey>,
	shka: BobKeyBundle,
	snhkb: BobKeyBundle,
	address_rks: Vec<BobKeyBundle>,
	address_pks: Option<Vec<PublicKey>>,
	number_shares: usize,
}

#[derive(Serialize, Deserialize)]
struct ExBobBundle {
	enc_rk: BobKeyBundleBytes,
	enc_pk: Option<String>,
	shka: BobKeyBundleBytes,
	snhkb: BobKeyBundleBytes,
	address_rks: Vec<BobKeyBundleBytes>,
	address_pks: Option<Vec<String>>,
	number_shares: usize,
}

impl From<&BobBundle> for ExBobBundle {
	fn from(bb: &BobBundle) -> Self {
		let enc_rk = bb.enc_rk.borrow().into();
		let enc_pk = bb.enc_pk.map(|e| e.to_jwk_string());
		let shka = bb.shka.borrow().into();
		let snhkb = bb.snhkb.borrow().into();
		let address_rks = bb.address_rks.iter()
			.map(|e| e.into())
			.collect();
		let address_pks = bb.address_pks.as_ref().map(|e| e.iter()
			.map(|e| e.to_jwk_string())
			.collect());
		let number_shares = bb.number_shares;
		Self {
			enc_rk,
			enc_pk,
			shka,
			snhkb,
			address_rks,
			address_pks,
			number_shares
		}
	}
}

impl From<&ExBobBundle> for BobBundle {
	fn from(ex_bb: &ExBobBundle) -> Self {
		let enc_rk = ex_bb.enc_rk.borrow().into();
		let enc_pk = ex_bb.enc_pk.as_ref().map(|e| PublicKey::from_jwk_str(e).unwrap());
		let shka = ex_bb.shka.borrow().into();
		let snhkb = ex_bb.snhkb.borrow().into();
		let address_rks = ex_bb.address_rks.iter()
			.map(|e| e.into())
			.collect();
		let address_pks = ex_bb.address_pks.as_ref().map(|e| e.iter()
			.map(|b| PublicKey::from_jwk_str(b).unwrap())
			.collect());
		let number_shares = ex_bb.number_shares;
		Self {
			enc_rk,
			enc_pk,
			shka,
			snhkb,
			address_rks,
			address_pks,
			number_shares
		}
	}
}

impl BobBundle {
	pub fn init(&mut self, alice_bundle: &AliceBundle) -> anyhow::Result<(AliceBob, MagicRatchet)> {
		if self.number_shares != alice_bundle.number_shares {
			return Err(anyhow::Error::from(PartnerBundleError::NumberOfSharesDifferent(self.number_shares, alice_bundle.number_shares)))
		}
		let enc_rk = self.enc_rk.key_exchange(&alice_bundle.enc_rk);
		let shka = self.shka.key_exchange(&alice_bundle.shka);
		let snhkb = self.snhkb.key_exchange(&alice_bundle.snhkb);
		let share_number = self.number_shares;
		let address_rks = self.address_rks.iter().zip(alice_bundle.address_rks.iter())
			.map(|e| e.0.key_exchange(e.1))
			.collect();
		let (magic_ratchet, pk, pks) = MagicRatchet::init_bob(enc_rk, shka, snhkb, share_number, address_rks);
		self.enc_pk = Some(pk);
		self.address_pks = Some(pks);
		Ok((AliceBob::Bob, magic_ratchet))
	}

	pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
		let ex = ExBobBundle::from(self);
		Ok(bincode::serialize(&ex)?)
	}

	pub fn from_bytes(inp: &[u8]) -> anyhow::Result<Self> {
		let ex: ExBobBundle = bincode::deserialize(inp)?;
		Ok(ex.borrow().into())
	}

	pub fn strip(&self) -> Self {
		Self {
			enc_rk: self.enc_rk.strip(),
			enc_pk: self.enc_pk,
			shka: self.shka.strip(),
			snhkb: self.snhkb.strip(),
			address_rks: self.address_rks.iter().map(|e| e.strip()).collect(),
			address_pks: self.address_pks.clone(),
			number_shares: self.number_shares
		}
	}
}

pub struct HostBundle {
	pub bundle: AliceBundle,
}

impl HostBundle {
	pub fn new(db: Db, bundle: AliceBundle) -> Self {
		let bundle_bytes = bundle.to_bytes().unwrap();
		let _ = db.insert(HOST_BUNDLE_KEY, bundle_bytes);
		Self {
			bundle
		}
	}
}

#[test]
fn bundle_build() {
	let number_shares = 3;
	let partner = AliceBob::Alice;
	let identity_key = IdentityKey::default();
	let bundler = BundleBuilder::default()
		.number_shares(number_shares)
		.identity_key(identity_key.clone())
		.partner(partner)
		.build()
		.unwrap();

	let alice_bundle = match bundler {
		PartnerBundle::Alice(d) => d,
		PartnerBundle::Bob(_) => {
			panic!("This is all wrong!")
		}
	};

	assert_eq!(alice_bundle.number_shares, number_shares);

	let partner = AliceBob::Bob;
	let identity_key = IdentityKey::default();

	let bundler = BundleBuilder::default()
		.number_shares(number_shares)
		.identity_key(identity_key.clone())
		.partner(partner)
		.build()
		.unwrap();

	let mut bob_bundle = match bundler {
		PartnerBundle::Bob(d) => d,
		PartnerBundle::Alice(_) => {
			panic!("This is all wrong!")
		}
	};

	let (bob, bob_ratchet) = bob_bundle.init(&alice_bundle).unwrap();
	let mut bob_session = SessionBuilder::default()
		.magic_ratchet(bob_ratchet)
		.partner(bob)
		.build();

	let (alice, alice_ratchet) = alice_bundle.init(&bob_bundle).unwrap();
	let mut alice_session = SessionBuilder::default()
		.magic_ratchet(alice_ratchet)
		.partner(alice)
		.build();
	assert_ne!(bob_session, alice_session);

	let encrypted = alice_session.make_init_message().unwrap();
	bob_session.process_init_message(encrypted);
}