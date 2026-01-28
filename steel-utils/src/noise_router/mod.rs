//! Pumpkin-style noise router for vanilla-accurate terrain generation.
//!
//! This module contains the data-driven noise router system ported from Pumpkin,
//! which uses pre-generated noise function component stacks to match vanilla Minecraft.

#[allow(missing_docs)]
pub mod density_function;
#[allow(missing_docs)]
pub mod evaluator;
#[allow(missing_docs)]
pub mod types;
// noise_params is generated at build time from noise_parameters.json
pub use crate::generated_noise_params as noise_params;

// density_functions is generated at build time from density_function.json
pub use crate::generated_density_functions::*;

// Chunk-specific noise router modules
#[allow(missing_docs)]
pub mod chunk_density_function;
#[allow(missing_docs)]
pub mod chunk_noise_router;
#[allow(missing_docs)]
pub mod proto_noise_router;

// Terrain generation modules
#[allow(missing_docs)]
pub mod aquifer_sampler;
#[allow(missing_docs)]
pub mod block_sampler;
#[allow(missing_docs)]
pub mod fluid_level;
#[allow(missing_docs)]
pub mod ore_sampler;
#[allow(missing_docs)]
pub mod surface_height_sampler;

pub use evaluator::{NoisePos, NoiseRouterRuntime};
pub use noise_params::*;
pub use types::*;

// Re-export chunk noise router types
pub use chunk_density_function::{
    ChunkNoiseFunctionBuilderOptions, ChunkNoiseFunctionSampleOptions,
    ChunkSpecificNoiseFunctionComponent, SampleAction, WrapperData,
};
pub use chunk_noise_router::{
    ChunkNoiseDensityFunction, ChunkNoiseFunctionComponent, ChunkNoiseRouter,
    MutableChunkNoiseFunctionComponentImpl, StaticChunkNoiseFunctionComponentImpl,
};
pub use proto_noise_router::{
    DependentProtoNoiseFunctionComponent, DoublePerlinNoiseBuilder,
    IndependentProtoNoiseFunctionComponent, ProtoMultiNoiseRouter, ProtoNoiseFunctionComponent,
    ProtoNoiseRouter, ProtoNoiseRouters, ProtoSurfaceEstimator,
};

// Re-export density function types
pub use density_function::NoisePos as NoisePosTraitAlias;
pub use density_function::{
    IndexToNoisePos, NoiseFunctionComponentRange, PassThrough,
    StaticIndependentChunkNoiseFunctionComponentImpl, UnblendedNoisePos, Wrapper,
};

// Re-export terrain generation types
pub use aquifer_sampler::{
    AquiferBlocks, AquiferSampler, AquiferSamplerImpl, SeaLevelAquiferSampler, WorldAquiferSampler,
};
pub use block_sampler::{BlockStateSampler, ChainedBlockStateSampler};
pub use fluid_level::{
    FluidLevel, FluidLevelSampler, FluidLevelSamplerImpl, StandardChunkFluidLevelSampler,
    StaticFluidLevelSampler,
};
pub use ore_sampler::{OreBlocks, OreVeinSampler};
pub use surface_height_sampler::{
    SurfaceHeightEstimateSampler, SurfaceHeightSamplerBuilderOptions,
};

#[cfg(test)]
mod tests {
    use super::*;
    use density_function::{StaticIndependentChunkNoiseFunctionComponentImpl, UnblendedNoisePos};
    use proto_noise_router::{
        IndependentProtoNoiseFunctionComponent, ProtoNoiseFunctionComponent, ProtoNoiseRouters,
    };

    const SEED: u64 = 12345;

    #[test]
    fn test_proto_router_generation() {
        // Just verify we can generate the proto routers without panicking
        let proto = ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, SEED);

        // Verify the stacks have content
        assert!(!proto.noise.full_component_stack.is_empty());
        // TODO: surface_estimator will be populated after Extractor re-extraction
        assert!(!proto.multi_noise.full_component_stack.is_empty());

        // Verify key router indices are valid
        assert!(proto.noise.final_density < proto.noise.full_component_stack.len());
        assert!(proto.noise.depth < proto.noise.full_component_stack.len());
        assert!(proto.noise.erosion < proto.noise.full_component_stack.len());
    }

    #[test]
    fn test_proto_router_determinism() {
        // Two routers with same seed should be identical
        let proto1 = ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, SEED);
        let proto2 = ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, SEED);

        assert_eq!(
            proto1.noise.full_component_stack.len(),
            proto2.noise.full_component_stack.len()
        );
        assert_eq!(proto1.noise.final_density, proto2.noise.final_density);
    }

    #[test]
    fn test_proto_router_different_seeds() {
        let proto1 = ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, 12345);
        let proto2 = ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, 54321);

        // The stack structure should be the same (same components)
        assert_eq!(
            proto1.noise.full_component_stack.len(),
            proto2.noise.full_component_stack.len()
        );

        // But noise values should differ
        // Find a noise component and sample it
        let pos = UnblendedNoisePos::new(100, 64, 100);
        for (c1, c2) in proto1
            .noise
            .full_component_stack
            .iter()
            .zip(proto2.noise.full_component_stack.iter())
        {
            if let (
                ProtoNoiseFunctionComponent::Independent(
                    IndependentProtoNoiseFunctionComponent::Noise(n1),
                ),
                ProtoNoiseFunctionComponent::Independent(
                    IndependentProtoNoiseFunctionComponent::Noise(n2),
                ),
            ) = (c1, c2)
            {
                let v1 = n1.sample(&pos);
                let v2 = n2.sample(&pos);
                // Different seeds should produce different noise
                if (v1 - v2).abs() > 1e-6 {
                    return; // Test passes - found different values
                }
            }
        }
        panic!("All noise values were identical with different seeds");
    }

    #[test]
    fn test_base_noise_router_structure() {
        // Verify the base router has the expected structure
        assert!(
            !OVERWORLD_BASE_NOISE_ROUTER
                .noise
                .full_component_stack
                .is_empty()
        );
        assert!(
            !OVERWORLD_BASE_NOISE_ROUTER
                .surface_estimator
                .full_component_stack
                .is_empty()
        );
        assert!(
            !OVERWORLD_BASE_NOISE_ROUTER
                .multi_noise
                .full_component_stack
                .is_empty()
        );

        // Verify indices are within bounds
        let noise = &OVERWORLD_BASE_NOISE_ROUTER.noise;
        assert!(noise.final_density < noise.full_component_stack.len());
        assert!(noise.barrier_noise < noise.full_component_stack.len());
        assert!(noise.erosion < noise.full_component_stack.len());
        assert!(noise.depth < noise.full_component_stack.len());
    }

    #[test]
    fn test_constant_component() {
        let proto = ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, SEED);

        // Find a constant component in the stack (there should be some)
        for (i, component) in proto.noise.full_component_stack.iter().enumerate() {
            if let ProtoNoiseFunctionComponent::Independent(
                IndependentProtoNoiseFunctionComponent::Constant(c),
            ) = component
            {
                // Constants should return the same value everywhere
                let pos1 = UnblendedNoisePos::new(0, 0, 0);
                let pos2 = UnblendedNoisePos::new(1000, 200, -500);
                let v1 = c.sample(&pos1);
                let v2 = c.sample(&pos2);
                assert!(
                    (v1 - v2).abs() < 1e-10,
                    "Constant at index {i} should be constant: {v1} vs {v2}"
                );
                return; // Test passes
            }
        }
        panic!("No constant component found in stack");
    }

    #[test]
    fn test_y_clamped_gradient_component() {
        let proto = ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, SEED);

        // Find a YClampedGradient component
        for component in &proto.noise.full_component_stack {
            if let ProtoNoiseFunctionComponent::Independent(
                IndependentProtoNoiseFunctionComponent::ClampedYGradient(g),
            ) = component
            {
                // Y gradient should vary with Y but not with X/Z
                let v_low = g.sample(&UnblendedNoisePos::new(0, -64, 0));
                let v_high = g.sample(&UnblendedNoisePos::new(0, 320, 0));
                let v_same_y = g.sample(&UnblendedNoisePos::new(1000, -64, -500));

                // Different Y should give different values
                assert!(
                    (v_low - v_high).abs() > 0.1,
                    "Y gradient should vary with Y: {v_low} vs {v_high}"
                );

                // Same Y should give same values regardless of X/Z
                assert!(
                    (v_low - v_same_y).abs() < 1e-10,
                    "Y gradient should not vary with X/Z: {v_low} vs {v_same_y}"
                );
                return;
            }
        }
        panic!("No YClampedGradient component found in stack");
    }

    #[test]
    fn test_noise_component_variation() {
        let proto = ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, SEED);

        // Find a noise component
        for component in &proto.noise.full_component_stack {
            if let ProtoNoiseFunctionComponent::Independent(
                IndependentProtoNoiseFunctionComponent::Noise(n),
            ) = component
            {
                // Sample at different positions - noise should vary spatially
                let samples: Vec<f64> = (0..10)
                    .map(|i| {
                        let pos = UnblendedNoisePos::new(i * 100, 64, i * 100);
                        n.sample(&pos)
                    })
                    .collect();

                // Check that we have some variation
                let min = samples.iter().copied().fold(f64::INFINITY, f64::min);
                let max = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                assert!(
                    max - min > 0.01,
                    "Noise should have spatial variation: min={min}, max={max}"
                );
                return;
            }
        }
        panic!("No noise component found in stack");
    }
}
