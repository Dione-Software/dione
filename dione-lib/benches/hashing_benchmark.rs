extern crate dione_lib;

use criterion::{Criterion, criterion_group, criterion_main, Throughput};
use dione_lib::hashing::cryptographic::sha512_hash_bytes;


fn adler_bench(c: &mut Criterion) {
	{
		const SIZE: usize = 1_000;

		let mut group = c.benchmark_group("adler-1000b");
		group.throughput(Throughput::Bytes(SIZE as u64));
		group.bench_function("hash-1000", |bencher| {
			bencher.iter(|| {
				dione_lib::hashing::non_cryptographic::adler_hash_bytes(&[0xff; SIZE]);
			});
		});
	}
	{
		const SIZE: u64 = 1_000_000;

		let mut group = c.benchmark_group("adler-1MB");
		group.throughput(Throughput::Bytes(SIZE));
		group.bench_function("hash-1MB", |bencher| {
			bencher.iter(|| {
				dione_lib::hashing::non_cryptographic::adler_hash_bytes(&[0; SIZE as usize])
			});
		});
	}
}

fn seahash_bench(c: &mut Criterion) {
	{
		const SIZE: usize = 1_000;

		let mut group = c.benchmark_group("seahash-1000b");
		group.throughput(Throughput::Bytes(SIZE as u64));
		group.bench_function("hash-1000b", |bencher| {
			bencher.iter(|| dione_lib::hashing::non_cryptographic::seahash_hash_bytes(&[0; SIZE]));
		});
	}
	{
		const SIZE: usize = 1_000_000;

		let mut group = c.benchmark_group("seahash-1MB");
		group.throughput(Throughput::Bytes(SIZE as u64));
		group.bench_function("seahash-1MB", |bencher| {
			bencher.iter(|| dione_lib::hashing::non_cryptographic::seahash_hash_bytes(&[0; SIZE]));
		});
	}
}

fn sha512_bench(c: &mut Criterion) {
	{
		const SIZE: usize = 1_000;

		let mut group = c.benchmark_group("sha512-1000b");
		group.throughput(Throughput::Bytes(SIZE as u64));
		group.bench_function("sha512-1000b", |bencher| {
			bencher.iter(|| sha512_hash_bytes(&[0; SIZE]));
		});
	}
	{
		const SIZE: usize = 1_000_000;

		let mut group = c.benchmark_group("sha512-1MB");
		group.throughput(Throughput::Bytes(SIZE as u64));
		group.bench_function("sha512-1MB", |bencher| {
			bencher.iter(|| sha512_hash_bytes(&[0; SIZE]));
		});
	}
}

criterion_group!(benches, adler_bench, seahash_bench, sha512_bench);
criterion_main!(benches);
