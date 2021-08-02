use crate::cryptography::sharing::{SharingAlgorithm, SharingError};
use alloc::prelude::v1::Vec;
use rand_core::OsRng;
use rand::Rng;

use rand::seq::SliceRandom;
use core::ops::{Sub, Add};

#[derive(Default)]
pub struct BlockSharing;

impl SharingAlgorithm for BlockSharing {
	fn share(&self, data: &[u8], n: usize) -> Result<Vec<Vec<u8>>, SharingError> {

		let mut float_len_vals: Vec<f64> = alloc::vec::Vec::with_capacity(n);
		for _ in 0..n {
			float_len_vals.push(OsRng.gen_range(0.0..1.0));
		}
		let sum: f64 = float_len_vals.iter().sum();
		float_len_vals = float_len_vals.iter()
			.map(|e| e / sum)
			.collect();
		let content_len_float = data.len() as f64;
		let content_len = data.len();
		let mut block_lenghts: Vec<usize> = float_len_vals
			.iter()
			.map(|e|  e * content_len_float)
			.map(|e| e.round())
			.map(|e| e as usize)
			.collect();
		let mut between_sum: usize = block_lenghts.iter().sum();
		while between_sum != content_len {
			let selection = block_lenghts.choose_mut(&mut OsRng).unwrap();
			if between_sum > content_len {
				if selection == &mut 0 {
					continue
				}
				*selection = selection.sub(&1);
			} else {
				*selection = selection.add(&1);
			}
			between_sum = block_lenghts.iter().sum();
		}

		let mut new_block_lengths = Vec::with_capacity(block_lenghts.len());
		let mut tmp = 0;
		for e in block_lenghts.iter() {
			new_block_lengths.push(tmp);
			tmp += *e
		}

		let block_lengths: Vec<(usize, usize)> = new_block_lengths.iter()
			.enumerate()
			.map(|e| {
				(*e.1, *block_lenghts.get(e.0).unwrap())
			}).collect();

		let res = block_lengths
			.iter()
			.map(|e| {
				let (_, b) = data.split_at(e.0);
				let (res, _) = b.split_at(e.1);
				res.to_vec()
			})
			.collect();

		Ok(res)
	}

	fn reconstruct(&self, inp: &[Vec<u8>]) -> Result<Vec<u8>, SharingError> {
		let mut result = Vec::new();
		for slice in inp {
			result.append(&mut slice.to_vec())
		}
		Ok(result)
	}
}

#[test]
fn share_test() {
	let algo = BlockSharing::default();
	let inp = b"Hello World".to_vec();
	let shares = algo.share(&inp, 3).unwrap();

	let reconstructed = algo.reconstruct(&shares).unwrap();
	assert_eq!(inp, reconstructed)
}