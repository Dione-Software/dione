use alloc::vec::Vec;

use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use lz4_flex::block::DecompressError;

pub fn compress_lz4(data: &[u8]) -> Vec<u8> {
	compress_prepend_size(data)
}

pub fn decompress_lz4(data: &[u8]) -> Result<Vec<u8>, DecompressError> {
	decompress_size_prepended(data)
}

#[test]
fn comp_decomp() {
	let input: &[u8] = b"Hello World";
	let compressed = compress_lz4(input);
	let decompressed = decompress_lz4(&compressed).unwrap();
	assert_eq!(input, &decompressed)
}