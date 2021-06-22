use criterion::{criterion_group, criterion_main, Criterion, Throughput};
extern crate dione_lib;

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

criterion_group!(benches, adler_bench, seahash_bench);
criterion_main!(benches);
