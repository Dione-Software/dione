use hmac::{Hmac, Mac, NewMac};

use core::convert::TryInto;

#[cfg(test)]
use crate::cryptography::ratchet::kdf_root::gen_ck;
use ring_compat::digest::Sha512;

type HmacSha512 = Hmac<Sha512>;

pub fn kdf_ck(ck: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
	let mac = HmacSha512::new_from_slice(ck)
		.expect("Invalid Key Length");
	let result = mac.finalize().into_bytes();
	let (a, b) = result.split_at(32);
	(a.try_into()
		 .expect("Incorrect Length"),
	 b.try_into()
		 .expect("Incorrect Length"))
}

#[cfg(test)]
#[allow(dead_code)]
pub fn gen_mk() -> [u8; 32] {
	let ck = gen_ck();
	let (_, mk) = kdf_ck(&ck);
	mk
}

#[cfg(test)]
mod tests {
	use crate::cryptography::ratchet::kdf_root::gen_ck;
	use crate::cryptography::ratchet::kdf_chain::kdf_ck;

	#[test]
	fn kdf_chain_ratchet() {
		let ck = gen_ck();
		let (ck, mk1) = kdf_ck(&ck);
		let (_, mk2) = kdf_ck(&ck);
		assert_ne!(mk1, mk2)
	}
}