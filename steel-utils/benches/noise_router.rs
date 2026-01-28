#![allow(missing_docs)]
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use steel_utils::noise_router::{
    OVERWORLD_BASE_NOISE_ROUTER,
    chunk_density_function::{
        ChunkNoiseFunctionBuilderOptions, ChunkNoiseFunctionSampleOptions, SampleAction,
        WrapperData,
    },
    chunk_noise_router::ChunkNoiseRouter,
    density_function::{IndexToNoisePos, NoisePos, UnblendedNoisePos},
    proto_noise_router::ProtoNoiseRouters,
};

const SEED: u64 = 12345;
const MIN_Y: i32 = -64;
const MAX_Y: i32 = 320;
const CELL_WIDTH: i32 = 4;
const CELL_HEIGHT: i32 = 8;

/// Floor division matching vanilla Minecraft.
#[inline]
fn floor_div(a: i32, b: i32) -> i32 {
    let q = a / b;
    let r = a % b;
    if r != 0 && (a < 0) != (b < 0) {
        q - 1
    } else {
        q
    }
}

struct InterpolationIndexMapper {
    x: i32,
    z: i32,
    minimum_cell_y: i32,
    vertical_cell_block_count: i32,
}

impl IndexToNoisePos for InterpolationIndexMapper {
    fn at(
        &self,
        index: usize,
        sample_data: Option<&mut ChunkNoiseFunctionSampleOptions>,
    ) -> impl NoisePos + 'static {
        if let Some(sample_data) = sample_data {
            sample_data.cache_result_unique_id += 1;
            sample_data.fill_index = index;
        }
        let y = (index as i32 + self.minimum_cell_y) * self.vertical_cell_block_count;
        UnblendedNoisePos::new(self.x, y, self.z)
    }
}

fn bench_proto_router_generation(c: &mut Criterion) {
    c.bench_function("proto_noise_router_generation", |b| {
        b.iter(|| {
            black_box(ProtoNoiseRouters::generate(
                black_box(&OVERWORLD_BASE_NOISE_ROUTER),
                black_box(SEED),
            ));
        });
    });
}

fn bench_chunk_router_generation(c: &mut Criterion) {
    let proto_routers = ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, SEED);

    let height = MAX_Y - MIN_Y;
    let horizontal_cells = 16 / CELL_WIDTH;
    let vertical_cell_count = height / CELL_HEIGHT;
    let base_x = 0;
    let base_z = 0;

    let horizontal_biome_end = ((horizontal_cells + 1) * CELL_WIDTH) as usize >> 2;
    let builder_options = ChunkNoiseFunctionBuilderOptions::new(
        CELL_WIDTH as usize,
        CELL_HEIGHT as usize,
        vertical_cell_count as usize,
        (horizontal_cells + 1) as usize,
        base_x >> 2,
        base_z >> 2,
        horizontal_biome_end,
    );

    c.bench_function("chunk_noise_router_generation", |b| {
        b.iter(|| {
            black_box(ChunkNoiseRouter::generate(
                black_box(&proto_routers.noise),
                black_box(&builder_options),
            ));
        });
    });
}

fn bench_fill_interpolator_buffers(c: &mut Criterion) {
    let proto_routers = ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, SEED);

    let height = MAX_Y - MIN_Y;
    let horizontal_cells = 16 / CELL_WIDTH;
    let vertical_cell_count = height / CELL_HEIGHT;
    let base_x = 0;
    let base_z = 0;
    let minimum_cell_y = floor_div(MIN_Y, CELL_HEIGHT);

    let horizontal_biome_end = ((horizontal_cells + 1) * CELL_WIDTH) as usize >> 2;
    let builder_options = ChunkNoiseFunctionBuilderOptions::new(
        CELL_WIDTH as usize,
        CELL_HEIGHT as usize,
        vertical_cell_count as usize,
        (horizontal_cells + 1) as usize,
        base_x >> 2,
        base_z >> 2,
        horizontal_biome_end,
    );

    c.bench_function("fill_interpolator_buffers", |b| {
        b.iter(|| {
            let mut router = ChunkNoiseRouter::generate(&proto_routers.noise, &builder_options);
            let mut cache_fill_unique_id: u64 = 0;
            let mut cache_result_unique_id: u64 = 0;

            let start_cell_x = floor_div(base_x, CELL_WIDTH);
            let start_cell_z = floor_div(base_z, CELL_WIDTH);
            let x = start_cell_x * CELL_WIDTH;

            for cell_z in 0..=(16 / CELL_WIDTH) {
                let current_cell_z = start_cell_z + cell_z;
                let z = current_cell_z * CELL_WIDTH;
                cache_fill_unique_id += 1;

                let mapper = InterpolationIndexMapper {
                    x,
                    z,
                    minimum_cell_y,
                    vertical_cell_block_count: CELL_HEIGHT,
                };

                let mut options = ChunkNoiseFunctionSampleOptions::new(
                    false,
                    SampleAction::CellCaches(WrapperData::new(
                        0,
                        0,
                        0,
                        CELL_WIDTH as usize,
                        CELL_HEIGHT as usize,
                    )),
                    cache_result_unique_id,
                    cache_fill_unique_id,
                    0,
                );

                router.fill_interpolator_buffers(true, cell_z as usize, &mapper, &mut options);
                cache_result_unique_id = options.cache_result_unique_id;
            }

            black_box(router);
        });
    });
}

fn bench_density_sampling(c: &mut Criterion) {
    let proto_routers = ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, SEED);

    let height = MAX_Y - MIN_Y;
    let horizontal_cells = 16 / CELL_WIDTH;
    let vertical_cell_count = height / CELL_HEIGHT;
    let base_x = 0;
    let base_z = 0;

    let horizontal_biome_end = ((horizontal_cells + 1) * CELL_WIDTH) as usize >> 2;
    let builder_options = ChunkNoiseFunctionBuilderOptions::new(
        CELL_WIDTH as usize,
        CELL_HEIGHT as usize,
        vertical_cell_count as usize,
        (horizontal_cells + 1) as usize,
        base_x >> 2,
        base_z >> 2,
        horizontal_biome_end,
    );

    let mut router = ChunkNoiseRouter::generate(&proto_routers.noise, &builder_options);

    c.bench_function("density_sample_single", |b| {
        b.iter(|| {
            let pos = UnblendedNoisePos::new(black_box(100), black_box(64), black_box(100));
            let sample_options = ChunkNoiseFunctionSampleOptions::new(
                false,
                SampleAction::CellCaches(WrapperData::new(0, 0, 0, 4, 8)),
                0,
                0,
                0,
            );
            black_box(router.final_density(&pos, &sample_options));
        });
    });
}

criterion_group!(
    benches,
    bench_proto_router_generation,
    bench_chunk_router_generation,
    bench_fill_interpolator_buffers,
    bench_density_sampling,
);
criterion_main!(benches);
