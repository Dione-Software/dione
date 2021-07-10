extern crate dione_lib;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};

use dione_lib::cryptography::symetric::aes_aead::{AesGcm, AesGcmSiv};
use dione_lib::cryptography::symetric::AeadCipher;

fn aes_gcm_bench(c: &mut Criterion) {
	let key = b"This is just an encryption key .";
	let nonce = b"This nonce i";
	{
		const SIZE: usize = 1_000;

		let mut group = c.benchmark_group("aes_gcm-1KB-encryption");
		group.throughput(Throughput::Bytes(SIZE as u64));
		group.bench_function("aes_gcm-1000", |bencher| {
			bencher.iter(|| {
				AesGcm::encrypt(&[0_u8; SIZE], key, nonce).unwrap();
			});
		});
	}
	{
		const SIZE: usize = 1_000_000;

		let mut group = c.benchmark_group("aes_gcm-1MB-encryption");
		group.throughput(Throughput::Bytes(SIZE as u64));
		group.bench_function("aes_gcm-1.000.000", |bencher| {
			bencher.iter(|| {
				AesGcm::encrypt(&[0_u8; SIZE], key, nonce).unwrap();
			});
		});
	}
	{
		const SIZE: usize = 1_000;

		let encrypted = AesGcm::encrypt(&[0_u8; SIZE], key, nonce).unwrap();

		let mut group = c.benchmark_group("aes_gcm-1KB-decryption");
		group.throughput(Throughput::Bytes(SIZE as u64));
		group.bench_function("aes_gcm-1.000", |bencher| {
			bencher.iter(|| {
				AesGcm::decrypt(&encrypted, key, nonce).unwrap();
			});
		});
	}
	{
		const SIZE: usize = 1_000_000;

		let encrypted = AesGcm::encrypt(&[0_u8; SIZE], key, nonce).unwrap();

		let mut group = c.benchmark_group("aes_gcm-1MB-decryption");
		group.throughput(Throughput::Bytes(SIZE as u64));
		group.bench_function("aes_gcm-1.000.000", |bencher| {
			bencher.iter(|| {
				AesGcm::decrypt(&encrypted, key, nonce).unwrap();
			});
		});
	}
}

fn aes_gcm_siv_bench(c: &mut Criterion) {
	let key = b"This is just an encryption key .";
	let nonce = b"This nonce i";
	{
		const SIZE: usize = 1_000;

		let mut group = c.benchmark_group("aes_gcm_siv-1KB-encryption");
		group.throughput(Throughput::Bytes(SIZE as u64));
		group.bench_function("aes_gcm_siv-1.000", |bencher| {
			bencher.iter(|| {
				AesGcmSiv::encrypt(&[0_u8; SIZE], key, nonce).unwrap();
			});
		});
	}
	{
		const SIZE: usize = 1_000_000;

		let mut group = c.benchmark_group("aes_gcm_siv-1MB-encryption");
		group.throughput(Throughput::Bytes(SIZE as u64));
		group.bench_function("aes_gcm_siv-1.000.000", |bencher| {
			bencher.iter(|| {
				AesGcmSiv::encrypt(&[0_u8; SIZE], key, nonce).unwrap();
			});
		});
	}
	{
		const SIZE: usize = 1_000;

		let encrypted = AesGcmSiv::encrypt(&[0_u8; SIZE], key, nonce).unwrap();

		let mut group = c.benchmark_group("aes_gcm_siv-1KB-decryption");
		group.throughput(Throughput::Bytes(SIZE as u64));
		group.bench_function("aes_gcm_siv-1.000", |bencher| {
			bencher.iter(|| {
				AesGcmSiv::decrypt(&encrypted, key, nonce).unwrap();
			});
		});
	}
	{
		const SIZE: usize = 1_000_000;

		let encrypted = AesGcmSiv::encrypt(&[0_u8; SIZE], key, nonce).unwrap();

		let mut group = c.benchmark_group("aes_gcm_siv-1MB-decryption");
		group.throughput(Throughput::Bytes(SIZE as u64));
		group.bench_function("aes_gcm_siv-1.000.000", |bencher| {
			bencher.iter(|| {
				AesGcmSiv::decrypt(&encrypted, key, nonce).unwrap();
			});
		});
	}
}

criterion_group!(benches, aes_gcm_bench);
criterion_main!(benches);