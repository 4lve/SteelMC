//! Full vanilla TerrainProvider implementation.
//!
//! This module implements the exact spline system used by vanilla Minecraft
//! for terrain shaping. The splines control offset (base height), factor
//! (height variation), and jaggedness (peak sharpness).

// Spline code uses mathematical single-letter variables (f, g, h, i, j)
#![allow(clippy::many_single_char_names)]

use super::{CubicSpline, DensityFn, SplineBuilder};

/// Compute peaks and valleys value from weirdness (ridges).
/// This is used to fold ridges into a [-1, 1] range for spline sampling.
#[must_use]
pub fn peaks_and_valleys(weirdness: f32) -> f32 {
    -(weirdness.abs() - 0.666_666_7).abs() - 0.333_333_34
}

/// Creates the overworld offset spline.
/// This controls the base terrain height.
pub fn overworld_offset(
    continents: DensityFn,
    erosion: DensityFn,
    ridges_folded: DensityFn,
    amplified: bool,
) -> CubicSpline {
    let transform: fn(f32) -> f32 = if amplified {
        |f| if f < 0.0 { f } else { f * 2.0 }
    } else {
        |f| f
    };

    let spline1 = build_erosion_offset_spline(
        erosion.clone(),
        ridges_folded.clone(),
        -0.15,
        0.0,
        0.0,
        0.1,
        0.0,
        -0.03,
        false,
        false,
        transform,
    );
    let spline2 = build_erosion_offset_spline(
        erosion.clone(),
        ridges_folded.clone(),
        -0.1,
        0.03,
        0.1,
        0.1,
        0.01,
        -0.03,
        false,
        false,
        transform,
    );
    let spline3 = build_erosion_offset_spline(
        erosion.clone(),
        ridges_folded.clone(),
        -0.1,
        0.03,
        0.1,
        0.7,
        0.01,
        -0.03,
        true,
        true,
        transform,
    );
    let spline4 = build_erosion_offset_spline(
        erosion,
        ridges_folded,
        -0.05,
        0.03,
        0.1,
        1.0,
        0.01,
        0.01,
        true,
        true,
        transform,
    );

    SplineBuilder::new(continents)
        .add_point(-1.1, apply_transform(0.044, transform), 0.0)
        .add_point(-1.02, apply_transform(-0.222_2, transform), 0.0)
        .add_point(-0.51, apply_transform(-0.222_2, transform), 0.0)
        .add_point(-0.44, apply_transform(-0.12, transform), 0.0)
        .add_point(-0.18, apply_transform(-0.12, transform), 0.0)
        .add_spline(-0.16, spline1.clone(), 0.0)
        .add_spline(-0.15, spline1, 0.0)
        .add_spline(-0.1, spline2, 0.0)
        .add_spline(0.25, spline3, 0.0)
        .add_spline(1.0, spline4, 0.0)
        .build()
}

/// Creates the overworld factor spline.
/// This controls the terrain height variation/scale.
pub fn overworld_factor(
    continents: DensityFn,
    erosion: DensityFn,
    ridges: DensityFn,
    ridges_folded: DensityFn,
    amplified: bool,
) -> CubicSpline {
    let transform: fn(f32) -> f32 = if amplified {
        |f| 1.25 - 6.25 / (f + 5.0)
    } else {
        |f| f
    };

    SplineBuilder::new(continents)
        .add_point(-0.19, 3.95, 0.0)
        .add_spline(
            -0.15,
            get_erosion_factor(
                erosion.clone(),
                ridges.clone(),
                ridges_folded.clone(),
                6.25,
                true,
                |f| f, // NO_TRANSFORM for this point
            ),
            0.0,
        )
        .add_spline(
            -0.1,
            get_erosion_factor(
                erosion.clone(),
                ridges.clone(),
                ridges_folded.clone(),
                5.47,
                true,
                transform,
            ),
            0.0,
        )
        .add_spline(
            0.03,
            get_erosion_factor(
                erosion.clone(),
                ridges.clone(),
                ridges_folded.clone(),
                5.08,
                true,
                transform,
            ),
            0.0,
        )
        .add_spline(
            0.06,
            get_erosion_factor(erosion, ridges, ridges_folded, 4.69, false, transform),
            0.0,
        )
        .build()
}

/// Creates the overworld jaggedness spline.
/// This controls the sharpness of mountain peaks.
pub fn overworld_jaggedness(
    continents: DensityFn,
    erosion: DensityFn,
    ridges: DensityFn,
    ridges_folded: DensityFn,
    amplified: bool,
) -> CubicSpline {
    let transform: fn(f32) -> f32 = if amplified { |f| f * 2.0 } else { |f| f };

    SplineBuilder::new(continents)
        .add_point(-0.11, 0.0, 0.0)
        .add_spline(
            0.03,
            build_erosion_jaggedness_spline(
                erosion.clone(),
                ridges.clone(),
                ridges_folded.clone(),
                1.0,
                0.5,
                0.0,
                0.0,
                transform,
            ),
            0.0,
        )
        .add_spline(
            0.65,
            build_erosion_jaggedness_spline(
                erosion,
                ridges,
                ridges_folded,
                1.0,
                1.0,
                1.0,
                0.0,
                transform,
            ),
            0.0,
        )
        .build()
}

#[allow(clippy::too_many_arguments)] // Spline building requires multiple control point values
fn build_erosion_jaggedness_spline(
    erosion: DensityFn,
    ridges: DensityFn,
    ridges_folded: DensityFn,
    high_erosion_high_weirdness: f32,
    low_erosion_high_weirdness: f32,
    high_erosion_mid_weirdness: f32,
    low_erosion_mid_weirdness: f32,
    transform: fn(f32) -> f32,
) -> CubicSpline {
    let spline1 = build_ridge_jaggedness_spline(
        ridges.clone(),
        ridges_folded.clone(),
        high_erosion_high_weirdness,
        high_erosion_mid_weirdness,
        transform,
    );
    let spline2 = build_ridge_jaggedness_spline(
        ridges,
        ridges_folded,
        low_erosion_high_weirdness,
        low_erosion_mid_weirdness,
        transform,
    );

    SplineBuilder::new(erosion)
        .add_spline(-1.0, spline1, 0.0)
        .add_spline(-0.78, spline2.clone(), 0.0)
        .add_spline(-0.577_5, spline2, 0.0)
        .add_point(-0.375, 0.0, 0.0)
        .build()
}

fn build_ridge_jaggedness_spline(
    ridges: DensityFn,
    ridges_folded: DensityFn,
    high_weirdness_magnitude: f32,
    mid_weirdness_magnitude: f32,
    transform: fn(f32) -> f32,
) -> CubicSpline {
    let pv_low = peaks_and_valleys(0.4);
    let pv_high = peaks_and_valleys(0.566_666_66);
    let pv_mid = f32::midpoint(pv_low, pv_high);

    let mut builder = SplineBuilder::new(ridges_folded);
    builder = builder.add_point(pv_low, 0.0, 0.0);

    if mid_weirdness_magnitude > 0.0 {
        builder = builder.add_spline(
            pv_mid,
            build_weirdness_jaggedness_spline(ridges.clone(), mid_weirdness_magnitude, transform),
            0.0,
        );
    } else {
        builder = builder.add_point(pv_mid, 0.0, 0.0);
    }

    if high_weirdness_magnitude > 0.0 {
        builder = builder.add_spline(
            1.0,
            build_weirdness_jaggedness_spline(ridges, high_weirdness_magnitude, transform),
            0.0,
        );
    } else {
        builder = builder.add_point(1.0, 0.0, 0.0);
    }

    builder.build()
}

fn build_weirdness_jaggedness_spline(
    ridges: DensityFn,
    magnitude: f32,
    transform: fn(f32) -> f32,
) -> CubicSpline {
    let f = 0.63 * magnitude;
    let g = 0.3 * magnitude;

    SplineBuilder::new(ridges)
        .add_point(-0.01, apply_transform(f, transform), 0.0)
        .add_point(0.01, apply_transform(g, transform), 0.0)
        .build()
}

fn get_erosion_factor(
    erosion: DensityFn,
    ridges: DensityFn,
    ridges_folded: DensityFn,
    value: f32,
    higher_values: bool,
    transform: fn(f32) -> f32,
) -> CubicSpline {
    let ridge_spline1 = SplineBuilder::new(ridges.clone())
        .add_point(-0.2, apply_transform(6.3, transform), 0.0)
        .add_point(0.2, apply_transform(value, transform), 0.0)
        .build();

    let ridge_spline2 = SplineBuilder::new(ridges.clone())
        .add_point(-0.05, apply_transform(6.3, transform), 0.0)
        .add_point(0.05, apply_transform(2.67, transform), 0.0)
        .build();

    let ridge_spline3 = SplineBuilder::new(ridges.clone())
        .add_point(-0.05, apply_transform(2.67, transform), 0.0)
        .add_point(0.05, apply_transform(6.3, transform), 0.0)
        .build();

    let mut builder = SplineBuilder::new(erosion)
        .add_spline(-0.6, ridge_spline1.clone(), 0.0)
        .add_spline(-0.5, ridge_spline2, 0.0)
        .add_spline(-0.35, ridge_spline1.clone(), 0.0)
        .add_spline(-0.25, ridge_spline1.clone(), 0.0)
        .add_spline(-0.1, ridge_spline3, 0.0)
        .add_spline(0.03, ridge_spline1.clone(), 0.0);

    if higher_values {
        let ridge_spline4 = SplineBuilder::new(ridges.clone())
            .add_point(0.0, apply_transform(value, transform), 0.0)
            .add_point(0.1, apply_transform(0.625, transform), 0.0)
            .build();

        let folded_spline = SplineBuilder::new(ridges_folded)
            .add_point(-0.9, apply_transform(value, transform), 0.0)
            .add_spline(-0.69, ridge_spline4, 0.0)
            .build();

        builder = builder
            .add_point(0.35, apply_transform(value, transform), 0.0)
            .add_spline(0.45, folded_spline.clone(), 0.0)
            .add_spline(0.55, folded_spline, 0.0)
            .add_point(0.62, apply_transform(value, transform), 0.0);
    } else {
        let folded_spline1 = SplineBuilder::new(ridges_folded.clone())
            .add_spline(-0.7, ridge_spline1.clone(), 0.0)
            .add_point(-0.15, apply_transform(1.37, transform), 0.0)
            .build();

        let folded_spline2 = SplineBuilder::new(ridges_folded)
            .add_spline(0.45, ridge_spline1.clone(), 0.0)
            .add_point(0.7, apply_transform(1.56, transform), 0.0)
            .build();

        builder = builder
            .add_spline(0.05, folded_spline2.clone(), 0.0)
            .add_spline(0.4, folded_spline2, 0.0)
            .add_spline(0.45, folded_spline1.clone(), 0.0)
            .add_spline(0.55, folded_spline1, 0.0)
            .add_point(0.58, apply_transform(value, transform), 0.0);
    }

    builder.build()
}

#[allow(clippy::too_many_arguments)] // Spline building requires multiple control point values
fn build_erosion_offset_spline(
    erosion: DensityFn,
    ridges_folded: DensityFn,
    ridge_base_offset: f32,
    ridge_mid_offset: f32,
    ridge_peak_offset: f32,
    magnitude: f32,
    ridge_inner_offset: f32,
    ridge_outer_offset: f32,
    extended: bool,
    use_max_slope: bool,
    transform: fn(f32) -> f32,
) -> CubicSpline {
    let spline1 = build_mountain_ridge_spline_with_points(
        ridges_folded.clone(),
        lerp(magnitude, 0.6, 1.5),
        use_max_slope,
        transform,
    );
    let spline2 = build_mountain_ridge_spline_with_points(
        ridges_folded.clone(),
        lerp(magnitude, 0.6, 1.0),
        use_max_slope,
        transform,
    );
    let spline3 = build_mountain_ridge_spline_with_points(
        ridges_folded.clone(),
        magnitude,
        use_max_slope,
        transform,
    );

    let spline4 = ridge_spline(
        ridges_folded.clone(),
        ridge_base_offset - 0.15,
        0.5 * magnitude,
        lerp(0.5, 0.5, 0.5) * magnitude,
        0.5 * magnitude,
        0.6 * magnitude,
        0.5,
        transform,
    );
    let spline5 = ridge_spline(
        ridges_folded.clone(),
        ridge_base_offset,
        ridge_inner_offset * magnitude,
        ridge_mid_offset * magnitude,
        0.5 * magnitude,
        0.6 * magnitude,
        0.5,
        transform,
    );
    let spline6 = ridge_spline(
        ridges_folded.clone(),
        ridge_base_offset,
        ridge_inner_offset,
        ridge_inner_offset,
        ridge_mid_offset,
        ridge_peak_offset,
        0.5,
        transform,
    );
    let spline7 = ridge_spline(
        ridges_folded.clone(),
        ridge_base_offset,
        ridge_inner_offset,
        ridge_inner_offset,
        ridge_mid_offset,
        ridge_peak_offset,
        0.5,
        transform,
    );
    let spline8 = SplineBuilder::new(ridges_folded.clone())
        .add_point(-1.0, apply_transform(ridge_base_offset, transform), 0.0)
        .add_spline(-0.4, spline6.clone(), 0.0)
        .add_point(
            0.0,
            apply_transform(ridge_peak_offset + 0.07, transform),
            0.0,
        )
        .build();
    let spline9 = ridge_spline(
        ridges_folded,
        -0.02,
        ridge_outer_offset,
        ridge_outer_offset,
        ridge_mid_offset,
        ridge_peak_offset,
        0.0,
        transform,
    );

    let mut builder = SplineBuilder::new(erosion)
        .add_spline(-0.85, spline1, 0.0)
        .add_spline(-0.7, spline2, 0.0)
        .add_spline(-0.4, spline3, 0.0)
        .add_spline(-0.35, spline4, 0.0)
        .add_spline(-0.1, spline5, 0.0)
        .add_spline(0.2, spline6, 0.0);

    if extended {
        builder = builder
            .add_spline(0.4, spline7.clone(), 0.0)
            .add_spline(0.45, spline8.clone(), 0.0)
            .add_spline(0.55, spline8, 0.0)
            .add_spline(0.58, spline7, 0.0);
    }

    builder.add_spline(0.7, spline9, 0.0).build()
}

fn build_mountain_ridge_spline_with_points(
    ridges_folded: DensityFn,
    magnitude: f32,
    use_max_slope: bool,
    transform: fn(f32) -> f32,
) -> CubicSpline {
    let h = mountain_continentalness(-1.0, magnitude, -0.7);
    let j = mountain_continentalness(1.0, magnitude, -0.7);
    let k = calculate_mountain_ridge_zero_continentalness_point(magnitude);

    let mut builder = SplineBuilder::new(ridges_folded);

    if -0.65 < k && k < 1.0 {
        let m = mountain_continentalness(-0.65, magnitude, -0.7);
        let o = mountain_continentalness(-0.75, magnitude, -0.7);
        let p = calculate_slope(h, o, -1.0, -0.75);
        builder = builder
            .add_point(-1.0, apply_transform(h, transform), p)
            .add_point(-0.75, apply_transform(o, transform), 0.0)
            .add_point(-0.65, apply_transform(m, transform), 0.0);

        let q = mountain_continentalness(k, magnitude, -0.7);
        let r = calculate_slope(q, j, k, 1.0);
        builder = builder
            .add_point(k - 0.01, apply_transform(q, transform), 0.0)
            .add_point(k, apply_transform(q, transform), r)
            .add_point(1.0, apply_transform(j, transform), r);
    } else {
        let m = calculate_slope(h, j, -1.0, 1.0);
        if use_max_slope {
            builder = builder
                .add_point(-1.0, apply_transform(0.2_f32.max(h), transform), 0.0)
                .add_point(0.0, apply_transform(lerp(0.5, h, j), transform), m);
        } else {
            builder = builder.add_point(-1.0, apply_transform(h, transform), m);
        }
        builder = builder.add_point(1.0, apply_transform(j, transform), m);
    }

    builder.build()
}

fn mountain_continentalness(height_factor: f32, magnitude: f32, cutoff_height: f32) -> f32 {
    let h = 1.0 - (1.0 - magnitude) * 0.5;
    let i = 0.5 * (1.0 - magnitude);
    let j = (height_factor + 1.17) * 0.460_829_47;
    let k = j * h - i;

    if height_factor < cutoff_height {
        k.max(-0.222_2)
    } else {
        k.max(0.0)
    }
}

fn calculate_mountain_ridge_zero_continentalness_point(magnitude: f32) -> f32 {
    let h = 1.0 - (1.0 - magnitude) * 0.5;
    let i = 0.5 * (1.0 - magnitude);
    i / (0.460_829_47 * h) - 1.17
}

#[allow(clippy::too_many_arguments)] // Spline building requires multiple control point values
fn ridge_spline(
    ridges_folded: DensityFn,
    y1: f32,
    y2: f32,
    y3: f32,
    y4: f32,
    y5: f32,
    min_smoothing: f32,
    transform: fn(f32) -> f32,
) -> CubicSpline {
    let f = (0.5 * (y2 - y1)).max(min_smoothing);
    let g = 5.0 * (y3 - y2);

    SplineBuilder::new(ridges_folded)
        .add_point(-1.0, apply_transform(y1, transform), f)
        .add_point(-0.4, apply_transform(y2, transform), f.min(g))
        .add_point(0.0, apply_transform(y3, transform), g)
        .add_point(0.4, apply_transform(y4, transform), 2.0 * (y4 - y3))
        .add_point(1.0, apply_transform(y5, transform), 0.7 * (y5 - y4))
        .build()
}

fn calculate_slope(y1: f32, y2: f32, x1: f32, x2: f32) -> f32 {
    (y2 - y1) / (x2 - x1)
}

fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + t * (b - a)
}

fn apply_transform(value: f32, transform: fn(f32) -> f32) -> f32 {
    transform(value)
}
