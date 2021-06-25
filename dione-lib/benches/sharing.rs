extern crate dione_lib;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use dione_lib::cryptography::sharing::SharingAlgorithm;

fn samir_bench(c: &mut Criterion) {
    const T: u8 = 5;
    const N: usize = 10;
    {
        const SIZE: usize = 1_000;

        let mut group = c.benchmark_group("samir-1000b");
        group.throughput(Throughput::Bytes(SIZE as u64));
        group.bench_function("samir-1000b", |bencher| {
            bencher.iter(|| {
                let algo = dione_lib::cryptography::sharing::shamir::ShamirSecretSharingShark;
                let shares = algo.share(&[0; SIZE], N, T);
                algo.reconstruct(&shares, T).unwrap();
            });
        });
    }
    {
        const SIZE: usize = 1_000_000;

        let mut group = c.benchmark_group("samir-1MB");
        group.sample_size(10);
        group.throughput(Throughput::Bytes(SIZE as u64));
        group.bench_function("samir-1MB", |bencher| {
            bencher.iter(|| {
                let algo = dione_lib::cryptography::sharing::shamir::ShamirSecretSharingShark;
                let shares = algo.share(&[0; SIZE], N, T);
                algo.reconstruct(&shares, T).unwrap();
            });
        });
    }
}

criterion_group!(benches, samir_bench);
criterion_main!(benches);
