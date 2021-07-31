use double_ratchet_2::ratchet::RatchetEncHeader;

mod header;
mod kdf_chain;
mod kdf_root;
mod address_ratchet;

pub struct MagicRatchet {
	enc_ratchet: RatchetEncHeader,
}