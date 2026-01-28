#![allow(missing_docs)]
//! Benchmarks for chunk generation.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use steel_core::chunk::{
    chunk_access::ChunkAccess,
    chunk_generator::ChunkGenerator,
    chunk_noise_generator::TerrainBlocks,
    proto_chunk::ProtoChunk,
    section::{ChunkSection, Sections},
    vanilla_noise_generator::VanillaNoiseGenerator,
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

/// Creates the terrain blocks configuration for benchmarks.
fn create_terrain_blocks() -> TerrainBlocks {
    TerrainBlocks {
        stone: BlockStateId(1),
        water: BlockStateId(80),
        lava: BlockStateId(81),
        air: BlockStateId(0),
        bedrock: BlockStateId(33),
        copper_ore: BlockStateId(850),
        deepslate_copper_ore: BlockStateId(851),
        raw_copper_block: BlockStateId(852),
        granite: BlockStateId(2),
        iron_ore: BlockStateId(849),
        deepslate_iron_ore: BlockStateId(850),
        raw_iron_block: BlockStateId(851),
        tuff: BlockStateId(853),
    }
}

fn bench_fill_chunk(c: &mut Criterion) {
    let blocks = create_terrain_blocks();
    let generator = VanillaNoiseGenerator::new(SEED, blocks);

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

fn bench_generator_creation(c: &mut Criterion) {
    c.bench_function("generator_creation", |b| {
        b.iter(|| {
            let blocks = create_terrain_blocks();
            let generator = VanillaNoiseGenerator::new(black_box(SEED), blocks);
            black_box(generator);
        });
    });
}

fn bench_multiple_chunks(c: &mut Criterion) {
    let blocks = create_terrain_blocks();
    let generator = VanillaNoiseGenerator::new(SEED, blocks);

    let mut group = c.benchmark_group("multiple_chunks");

    // Benchmark generating multiple adjacent chunks (simulates player movement)
    for count in [4, 9, 16] {
        group.bench_with_input(
            BenchmarkId::new("adjacent", count),
            &count,
            |b, &count| {
                let side = (count as f64).sqrt() as i32;
                b.iter(|| {
                    for x in 0..side {
                        for z in 0..side {
                            let chunk = create_empty_chunk(x, z);
                            generator.fill_from_noise(black_box(&chunk));
                            black_box(&chunk);
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_fill_chunk,
    bench_generator_creation,
    bench_multiple_chunks,
);
criterion_main!(benches);
