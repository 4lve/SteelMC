#![allow(missing_docs)]
//! Benchmarks for chunk generation.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use steel_core::chunk::{
    chunk_access::ChunkAccess,
    chunk_generator::ChunkGenerator,
    noise_chunk_generator::NoiseChunkGenerator,
    proto_chunk::ProtoChunk,
    section::{ChunkSection, Sections},
};
use steel_utils::{BlockStateId, ChunkPos, math::Vector2};

const SEED: u64 = 12345;
const MIN_Y: i32 = -64;
const MAX_Y: i32 = 320;
const HEIGHT: i32 = MAX_Y - MIN_Y;
const NUM_SECTIONS: usize = (HEIGHT / 16) as usize;

/// Creates an empty proto chunk at the given position.
fn create_empty_chunk(x: i32, z: i32) -> ChunkAccess {
    let sections: Box<[ChunkSection]> = (0..NUM_SECTIONS)
        .map(|_| ChunkSection::new_empty())
        .collect();
    let sections = Sections::from_owned(sections);
    let pos = ChunkPos(Vector2::new(x, z));
    ChunkAccess::Proto(ProtoChunk::new(sections, pos, MIN_Y, HEIGHT))
}

fn bench_sample_cell_corners(c: &mut Criterion) {
    let generator = NoiseChunkGenerator::new(
        SEED,
        BlockStateId(1), // stone
        BlockStateId(2), // water
        BlockStateId(3), // bedrock
        BlockStateId(4), // deepslate
    );

    c.bench_function("single_density_sample", |b| {
        b.iter(|| {
            // Sample at origin chunk
            black_box(generator.compute_density(black_box(0), black_box(64), black_box(0)));
        });
    });

    // Benchmark full cell corner sampling for a chunk
    // This samples 5x5x49 = 1225 corner positions
    let mut group = c.benchmark_group("cell_corner_sampling");

    let positions = [(0, 0), (100, 100), (1000, 1000)];

    for (x, z) in positions {
        group.bench_with_input(
            BenchmarkId::new("chunk", format!("({x},{z})")),
            &(x, z),
            |b, &(x, z)| {
                let base_x = x * 16;
                let base_z = z * 16;
                b.iter(|| {
                    // Sample all cell corners (5x5x49 = 1225 samples)
                    let mut total = 0.0;
                    for cx in 0..5 {
                        for cz in 0..5 {
                            for cy in 0..49 {
                                let world_x = base_x + cx * 4;
                                let world_y = -64 + cy * 8;
                                let world_z = base_z + cz * 4;
                                total += generator.compute_density(
                                    black_box(world_x),
                                    black_box(world_y),
                                    black_box(world_z),
                                );
                            }
                        }
                    }
                    black_box(total);
                });
            },
        );
    }

    group.finish();
}

fn bench_fill_chunk(c: &mut Criterion) {
    let generator = NoiseChunkGenerator::new(
        SEED,
        BlockStateId(1), // stone
        BlockStateId(2), // water
        BlockStateId(3), // bedrock
        BlockStateId(4), // deepslate
    );

    let mut group = c.benchmark_group("fill_from_noise");

    // Benchmark at different chunk positions to see variance
    let positions = [(0, 0), (100, 100), (1000, 1000)];

    for (x, z) in positions {
        group.bench_with_input(
            BenchmarkId::new("chunk", format!("({x},{z})")),
            &(x, z),
            |b, &(x, z)| {
                b.iter(|| {
                    let chunk = create_empty_chunk(x, z);
                    generator.fill_from_noise(black_box(&chunk));
                    black_box(chunk);
                });
            },
        );
    }

    group.finish();
}

fn bench_density_sampling(c: &mut Criterion) {
    let generator = NoiseChunkGenerator::new(
        SEED,
        BlockStateId(1),
        BlockStateId(2),
        BlockStateId(3),
        BlockStateId(4),
    );

    let mut group = c.benchmark_group("density_sampling");

    // Sample at different Y levels to see the optimization effect
    let y_levels = [
        (-60, "deep_underground"),
        (0, "sea_level"),
        (64, "surface"),
        (200, "high_altitude"),
    ];

    for (y, name) in y_levels {
        group.bench_with_input(BenchmarkId::new("y_level", name), &y, |b, &y| {
            b.iter(|| {
                let mut total = 0.0;
                for x in 0..16 {
                    for z in 0..16 {
                        total +=
                            generator.compute_density(black_box(x), black_box(y), black_box(z));
                    }
                }
                black_box(total);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_sample_cell_corners,
    bench_fill_chunk,
    bench_density_sampling,
);
criterion_main!(benches);
