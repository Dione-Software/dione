use x3dh_ke::{EphemeralKey, SignedPreKey, OneTimePreKey, x3dh_b, x3dh_a};
use p256::ecdsa::Signature;
use alloc::vec::Vec;
pub use x3dh_ke::{IdentityKey, Key};
#[cfg(test)]
use core::fmt::{Debug, Formatter};
use serde::{Serialize, Deserialize};
use zeroize::Zeroize;

pub struct AliceKeyBundle {
	ik: IdentityKey,
	ek: EphemeralKey,
}

impl AliceKeyBundle {
	pub fn new(ik: &IdentityKey) -> Self {

		Self {
			ik: ik.clone(),
			ek: EphemeralKey::default(),
		}
	}

	pub fn strip(&self) -> Self {
		Self {
			ik: self.ik.strip(),
			ek: self.ek.strip(),
		}
	}
}

#[test]
fn alice_strip_test() {
	let alice = AliceKeyBundle::new(&IdentityKey::default());
	let bob = BobKeyBundle::new(&IdentityKey::default());
	let alice_striped = alice.strip();
	let sk1 = bob.key_exchange(&alice);
	let sk2 = bob.key_exchange(&alice_striped);
	assert_eq!(sk1, sk2)
}

#[cfg(test)]
impl PartialEq for AliceKeyBundle {
	fn eq(&self, other: &Self) -> bool {
		self.ek.to_bytes() == other.ek.to_bytes() && self.ik.to_bytes() == other.ik.to_bytes()
	}
}

#[cfg(test)]
impl Debug for AliceKeyBundle {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Alice Key Bundle")
			.field("Identity Key", &self.ik.to_bytes())
			.field("Ephemeral Key", &self.ek.to_bytes())
			.finish()
	}
}

#[derive(Serialize, Deserialize, Zeroize)]
struct AliceKeyBundleBytes {
	#[serde(with = "serde_bytes")]
	ik: Vec<u8>,
	#[serde(with = "serde_bytes")]
	ek: Vec<u8>,
}

impl From<&AliceKeyBundle> for AliceKeyBundleBytes {
	fn from(akb: &AliceKeyBundle) -> Self {
		let ikb = akb.ik.to_bytes();
		let ekb = akb.ek.to_bytes();
		Self {
			ik: ikb,
			ek: ekb,
		}
	}
}

impl From<&AliceKeyBundleBytes> for AliceKeyBundle {
	fn from(akbb: &AliceKeyBundleBytes) -> Self {
		let ik = IdentityKey::from_bytes(&akbb.ik)
			.expect("Error parsing back to identity key");
		let ek = EphemeralKey::from_bytes(&akbb.ek)
			.expect("Error parsing back to ephemeral key");
		Self {
			ik,
			ek,
		}
	}
}

pub struct BobKeyBundle {
	ik: IdentityKey,
	spk: SignedPreKey,
	opk: OneTimePreKey,
	signature: Signature,
}

impl BobKeyBundle {
	pub fn new(ik: &IdentityKey) -> Self {
		let spk = SignedPreKey::default();
		let stripped_pre_key_bytes = &spk
			.strip()
			.pk_to_bytes();
		let opk = OneTimePreKey::default();
		let signature = ik.sign(stripped_pre_key_bytes);
		Self {
			ik: ik.clone(),
			spk,
			opk,
			signature
		}
	}
	pub fn strip(&self) -> Self {
		Self {
			ik: self.ik.strip(),
			spk: self.spk.strip(),
			opk: self.opk.strip(),
			signature: self.signature.clone(),
		}
	}

}

#[test]
fn bob_strip_test() {
	let bob = BobKeyBundle::new(&IdentityKey::default());
	let alice = AliceKeyBundle::new(&IdentityKey::default());
	let bob_striped = bob.strip();
	let sk1 = alice.key_exchange(&bob)
		.expect("Signature is invalid");
	let sk2 = alice.key_exchange(&bob_striped)
		.expect("Signature is invalid");
	assert_eq!(sk1, sk2)
}

#[cfg(test)]
impl PartialEq for BobKeyBundle {
	fn eq(&self, other: &Self) -> bool {
		self.ik.to_bytes() == other.ik.to_bytes() &&
			self.spk.to_bytes() == other.spk.to_bytes() &&
			self.opk.to_bytes() == other.opk.to_bytes() &&
			self.signature == other.signature
	}
}

#[cfg(test)]
impl Debug for BobKeyBundle {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Bob Key Bundle")
			.field("Identity Key", &self.ik.to_bytes())
			.field("Signed Pre Key", &self.spk.to_bytes())
			.field("One Time Pre Key", &self.opk.to_bytes())
			.field("Signature", &self.signature)
			.finish()
	}
}

#[derive(Serialize, Deserialize, Zeroize)]
pub struct BobKeyBundleBytes {
	#[serde(with = "serde_bytes")]
	ik: Vec<u8>,
	#[serde(with = "serde_bytes")]
	spk: Vec<u8>,
	#[serde(with = "serde_bytes")]
	opk: Vec<u8>,
	#[serde(with = "serde_bytes")]
	signature: Vec<u8>,
}

impl From<&BobKeyBundle> for BobKeyBundleBytes {
	fn from(bkb: &BobKeyBundle) -> Self {
		let ik = bkb.ik.to_bytes();
		let spk = bkb.spk.to_bytes();
		let opk = bkb.opk.to_bytes();
		let signature = bkb.signature.to_der().to_bytes().to_vec();
		Self {
			ik,
			spk,
			opk,
			signature
		}
	}
}

impl From<&BobKeyBundleBytes> for BobKeyBundle {
	fn from(bkbb: &BobKeyBundleBytes) -> Self {
		let ik = IdentityKey::from_bytes(&bkbb.ik)
			.expect("Error parsing identity key");
		let spk = SignedPreKey::from_bytes(&bkbb.spk)
			.expect("Error parsing signed pre-key");
		let opk = OneTimePreKey::from_bytes(&bkbb.opk)
			.expect("Error parsing one-time pre-key");
		let signature = Signature::from_der(&bkbb.signature)
			.expect("Error parsing signature");
		Self {
			ik,
			spk,
			opk,
			signature
		}
	}
}

#[cfg(test)]
mod alic_tests {
	use crate::cryptography::key_exchange::{IdentityKey, Key, AliceKeyBundle, AliceKeyBundleBytes};

	#[test]
	fn generate() {
		let identity_key = IdentityKey::default();
		let _alice_key_bundle = AliceKeyBundle::new(&identity_key);
	}

	#[test]
	fn alice_key_bundle_bytes() {
		let identity_key = IdentityKey::default();
		let akb = AliceKeyBundle::new(&identity_key);
		let akbb = AliceKeyBundleBytes::from(&akb);
		let akbr = AliceKeyBundle::from(&akbb);
		assert_eq!(akbr, akb)
	}

	#[test]
	fn ser_des() {
		let identity_key = IdentityKey::default();
		let akb = AliceKeyBundle::new(&identity_key);
		let akbb = AliceKeyBundleBytes::from(&akb);
		let bincoded = bincode::serialize(&akbb)
			.expect("error serializing");
		let re: AliceKeyBundleBytes = bincode::deserialize(&bincoded)
			.expect("error deserializing");
		let akbr = AliceKeyBundle::from(&re);
		assert_eq!(akbr, akb)
	}
}

#[cfg(test)]
mod bob_tests {
	use crate::cryptography::key_exchange::{IdentityKey, Key, BobKeyBundle, BobKeyBundleBytes};

	#[test]
	fn generate() {
		let identity_key = IdentityKey::default();
		let _ = BobKeyBundle::new(&identity_key);
	}

	#[test]
	fn bob_key_bundle_bytes() {
		let identity_key = IdentityKey::default();
		let bkb = BobKeyBundle::new(&identity_key);
		let bkbb = BobKeyBundleBytes::from(&bkb);
		let bkbr = BobKeyBundle::from(&bkbb);
		assert_eq!(bkbr, bkb)
	}

	#[test]
	fn ser_des() {
		let identity_key = IdentityKey::default();
		let bkb = BobKeyBundle::new(&identity_key);
		let bkbb = BobKeyBundleBytes::from(&bkb);
		let bincoded = bincode::serialize(&bkbb)
			.expect("Error serializing");
		let re: BobKeyBundleBytes = bincode::deserialize(&bincoded)
			.expect("Error deserializing");
		let bkbr = BobKeyBundle::from(&re);
		assert_eq!(bkb, bkbr)
	}
}

impl BobKeyBundle {
	pub fn key_exchange(&self, alice: &AliceKeyBundle) -> [u8; 32] {
		let ika = &alice.ik;
		let spkb = &self.spk;
		let eka = &alice.ek;
		let ikb = &self.ik;
		let opkb = &self.opk;
		x3dh_b(ika, spkb, eka, ikb, opkb)
	}
}

impl AliceKeyBundle {
	pub fn key_exchange(&self, bob: &BobKeyBundle) -> Result<[u8; 32], &'static str> {
		let signature = &bob.signature;
		let ika = &self.ik;
		let spkb = &bob.spk;
		let eka = &self.ek;
		let ikb = &bob.ik;
		let opkb = &bob.opk;
		x3dh_a(signature, ika, spkb, eka, ikb, opkb)
	}
}

#[cfg(test)]
mod key_echange {
	use crate::cryptography::key_exchange::{IdentityKey, AliceKeyBundle, Key, BobKeyBundle};

	#[test]
	fn key_exchange_unstriped() {
		let ika = IdentityKey::default();
		let ikb = IdentityKey::default();
		let alice_key_bundle = AliceKeyBundle::new(&ika);
		let bob_key_bundle = BobKeyBundle::new(&ikb);
		let cka = alice_key_bundle.key_exchange(&bob_key_bundle)
			.expect("Signature probably invalid");
		let ckb = bob_key_bundle.key_exchange(&alice_key_bundle);
		assert_eq!(cka, ckb)
	}

	#[test]
	fn key_exchange_striped() {
		let ika = IdentityKey::default();
		let ikb = IdentityKey::default();
		let alice = AliceKeyBundle::new(&ika);
		let bob = BobKeyBundle::new(&ikb);
		let alice_s = alice.strip();
		let bob_s = bob.strip();
		let sk1 = alice.key_exchange(&bob_s)
			.expect("Signature is invalid");
		let sk2 = bob.key_exchange(&alice_s);
		assert_eq!(sk1, sk2)
	}
}

