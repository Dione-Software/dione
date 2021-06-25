use crate::message_storage::HashType;
use crate::message_storage::SaveMessageRequest;
use dione_lib::hashing::cryptographic::sha512_hash_bytes;
use dione_lib::hashing::non_cryptographic::{adler_hash_bytes, seahash_hash_bytes};
use tracing::*;

impl SaveMessageRequest {
    #[instrument(skip(self))]
    pub(crate) fn hash_content(&self) -> (HashType, Vec<u8>) {
        trace!("Extracting content");
        let content = &self.content;
        let content_length = content.len();
        let hash_type = match content_length {
            l if (0..=1_000).contains(&l) => HashType::Sha512,
            l if (1_000..=10_000).contains(&l) => HashType::Adler32,
            _ => HashType::Lz4,
        };
        trace!("Hashing");
        let hash = match hash_type {
            HashType::Sha512 => sha512_hash_bytes,
            HashType::Adler32 => adler_hash_bytes,
            HashType::Lz4 => seahash_hash_bytes,
        }(content);
        (hash_type, hash)
    }
}
