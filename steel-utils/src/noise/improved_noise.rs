//! Improved Perlin noise implementation for vanilla-accurate world generation.
//!
//! This is an exact port of Minecraft's `ImprovedNoise` class.

// Noise code uses mathematical single-letter variables (x, y, z, i, j, k)
#![allow(clippy::many_single_char_names)]

use crate::random::{Random, RandomSource};

use super::{
    floor, lerp3,
    simplex_noise::{GRADIENT, dot},
    smoothstep,
};

/// Improved Perlin noise generator.
///
/// Implements classic Perlin noise with a permutation table,
/// matching Minecraft's `ImprovedNoise` exactly.
pub struct ImprovedNoise {
    /// Permutation table (256 bytes).
    p: [u8; 256],
    /// X offset for noise variation.
    pub xo: f64,
    /// Y offset for noise variation.
    pub yo: f64,
    /// Z offset for noise variation.
    pub zo: f64,
}

impl ImprovedNoise {
    /// Creates a new `ImprovedNoise` from a random source.
    pub fn new(random: &mut RandomSource) -> Self {
        let xo = random.next_f64() * 256.0;
        let yo = random.next_f64() * 256.0;
        let zo = random.next_f64() * 256.0;

        let mut p = [0u8; 256];

        // Initialize with identity
        for (i, item) in p.iter_mut().enumerate() {
            *item = i as u8;
        }

        // Fisher-Yates shuffle
        for i in 0..256 {
            let j = random.next_i32_bounded((256 - i) as i32) as usize;
            p.swap(i, i + j);
        }

        Self { p, xo, yo, zo }
    }

    /// Get permutation value with wrapping.
    #[inline]
    fn p(&self, index: i32) -> i32 {
        i32::from(self.p[(index & 255) as usize])
    }

    /// Compute gradient dot product.
    #[inline]
    fn grad_dot(grad_index: i32, x: f64, y: f64, z: f64) -> f64 {
        dot(GRADIENT[(grad_index & 15) as usize], x, y, z)
    }

    /// Sample 3D Perlin noise at the given coordinates.
    #[inline]
    #[must_use]
    pub fn noise(&self, x: f64, y: f64, z: f64) -> f64 {
        self.noise_with_y_params(x, y, z, 0.0, 0.0)
    }

    /// Sample 3D Perlin noise with Y-axis scaling parameters.
    /// Used by `BlendedNoise` for vertical smearing.
    #[inline]
    #[must_use]
    pub fn noise_with_y_params(&self, x: f64, y: f64, z: f64, y_scale: f64, y_max: f64) -> f64 {
        let d = x + self.xo;
        let e = y + self.yo;
        let f = z + self.zo;

        let i = floor(d);
        let j = floor(e);
        let k = floor(f);

        let g = d - f64::from(i);
        let h = e - f64::from(j);
        let l = f - f64::from(k);

        // Compute weird_delta_y for vertical interpolation
        // Vanilla: (double)Mth.floor(fudgeLimit / yScale + 1.0E-7F) * yScale
        // Must use integer floor then cast back to f64 to match vanilla
        let n = if y_scale == 0.0 {
            0.0
        } else {
            let m = if y_max >= 0.0 && y_max < h { y_max } else { h };
            // Use f32 then f64 to match vanilla's 1.0E-7F float literal
            f64::from(floor(m / y_scale + f64::from(1.0e-7_f32))) * y_scale
        };

        self.sample_and_lerp(i, j, k, g, h - n, l, h)
    }

    /// Sample noise with derivatives (for terrain normal computation).
    #[inline]
    #[must_use]
    pub fn noise_with_derivative(&self, x: f64, y: f64, z: f64, values: &mut [f64; 3]) -> f64 {
        let d = x + self.xo;
        let e = y + self.yo;
        let f = z + self.zo;

        let i = floor(d);
        let j = floor(e);
        let k = floor(f);

        let g = d - f64::from(i);
        let h = e - f64::from(j);
        let l = f - f64::from(k);

        self.sample_with_derivative(i, j, k, g, h, l, values)
    }

    /// Sample and interpolate noise at grid point.
    #[allow(clippy::too_many_arguments)] // Noise sampling needs grid position + interpolation deltas
    #[inline]
    fn sample_and_lerp(
        &self,
        grid_x: i32,
        grid_y: i32,
        grid_z: i32,
        delta_x: f64,
        weird_delta_y: f64,
        delta_z: f64,
        delta_y: f64,
    ) -> f64 {
        let i = self.p(grid_x);
        let j = self.p(grid_x + 1);
        let k = self.p(i + grid_y);
        let l = self.p(i + grid_y + 1);
        let m = self.p(j + grid_y);
        let n = self.p(j + grid_y + 1);

        let d = Self::grad_dot(self.p(k + grid_z), delta_x, weird_delta_y, delta_z);
        let e = Self::grad_dot(self.p(m + grid_z), delta_x - 1.0, weird_delta_y, delta_z);
        let f = Self::grad_dot(self.p(l + grid_z), delta_x, weird_delta_y - 1.0, delta_z);
        let g = Self::grad_dot(
            self.p(n + grid_z),
            delta_x - 1.0,
            weird_delta_y - 1.0,
            delta_z,
        );
        let h = Self::grad_dot(
            self.p(k + grid_z + 1),
            delta_x,
            weird_delta_y,
            delta_z - 1.0,
        );
        let o = Self::grad_dot(
            self.p(m + grid_z + 1),
            delta_x - 1.0,
            weird_delta_y,
            delta_z - 1.0,
        );
        let p = Self::grad_dot(
            self.p(l + grid_z + 1),
            delta_x,
            weird_delta_y - 1.0,
            delta_z - 1.0,
        );
        let q = Self::grad_dot(
            self.p(n + grid_z + 1),
            delta_x - 1.0,
            weird_delta_y - 1.0,
            delta_z - 1.0,
        );

        let r = smoothstep(delta_x);
        let s = smoothstep(delta_y);
        let t = smoothstep(delta_z);

        lerp3(r, s, t, d, e, f, g, h, o, p, q)
    }

    /// Sample noise with derivative computation.
    #[allow(clippy::too_many_arguments)] // Noise sampling needs grid position + interpolation deltas
    #[inline]
    fn sample_with_derivative(
        &self,
        grid_x: i32,
        grid_y: i32,
        grid_z: i32,
        delta_x: f64,
        delta_y: f64,
        delta_z: f64,
        noise_values: &mut [f64; 3],
    ) -> f64 {
        let i = self.p(grid_x);
        let j = self.p(grid_x + 1);
        let k = self.p(i + grid_y);
        let l = self.p(i + grid_y + 1);
        let m = self.p(j + grid_y);
        let n = self.p(j + grid_y + 1);

        let o = self.p(k + grid_z);
        let p = self.p(m + grid_z);
        let q = self.p(l + grid_z);
        let r = self.p(n + grid_z);
        let s = self.p(k + grid_z + 1);
        let t = self.p(m + grid_z + 1);
        let u = self.p(l + grid_z + 1);
        let v = self.p(n + grid_z + 1);

        let is = GRADIENT[(o & 15) as usize];
        let js = GRADIENT[(p & 15) as usize];
        let ks = GRADIENT[(q & 15) as usize];
        let ls = GRADIENT[(r & 15) as usize];
        let ms = GRADIENT[(s & 15) as usize];
        let ns = GRADIENT[(t & 15) as usize];
        let os = GRADIENT[(u & 15) as usize];
        let ps = GRADIENT[(v & 15) as usize];

        let d = dot(is, delta_x, delta_y, delta_z);
        let e = dot(js, delta_x - 1.0, delta_y, delta_z);
        let f = dot(ks, delta_x, delta_y - 1.0, delta_z);
        let g = dot(ls, delta_x - 1.0, delta_y - 1.0, delta_z);
        let h = dot(ms, delta_x, delta_y, delta_z - 1.0);
        let w = dot(ns, delta_x - 1.0, delta_y, delta_z - 1.0);
        let x = dot(os, delta_x, delta_y - 1.0, delta_z - 1.0);
        let y = dot(ps, delta_x - 1.0, delta_y - 1.0, delta_z - 1.0);

        let z = smoothstep(delta_x);
        let aa = smoothstep(delta_y);
        let ab = smoothstep(delta_z);

        let ac = lerp3(
            z,
            aa,
            ab,
            f64::from(is[0]),
            f64::from(js[0]),
            f64::from(ks[0]),
            f64::from(ls[0]),
            f64::from(ms[0]),
            f64::from(ns[0]),
            f64::from(os[0]),
            f64::from(ps[0]),
        );
        let ad = lerp3(
            z,
            aa,
            ab,
            f64::from(is[1]),
            f64::from(js[1]),
            f64::from(ks[1]),
            f64::from(ls[1]),
            f64::from(ms[1]),
            f64::from(ns[1]),
            f64::from(os[1]),
            f64::from(ps[1]),
        );
        let ae = lerp3(
            z,
            aa,
            ab,
            f64::from(is[2]),
            f64::from(js[2]),
            f64::from(ks[2]),
            f64::from(ls[2]),
            f64::from(ms[2]),
            f64::from(ns[2]),
            f64::from(os[2]),
            f64::from(ps[2]),
        );

        let af = super::lerp2(aa, ab, e - d, g - f, w - h, y - x);
        let ag = super::lerp2(ab, z, f - d, x - h, g - e, y - w);
        let ah = super::lerp2(z, aa, h - d, w - e, x - f, y - g);

        let ai = super::smoothstep_derivative(delta_x);
        let aj = super::smoothstep_derivative(delta_y);
        let ak = super::smoothstep_derivative(delta_z);

        noise_values[0] += ac + ai * af;
        noise_values[1] += ad + aj * ag;
        noise_values[2] += ae + ak * ah;

        lerp3(z, aa, ab, d, e, f, g, h, w, x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::xoroshiro::Xoroshiro;

    #[test]
    fn test_improved_noise_deterministic() {
        let rng1 = Xoroshiro::from_seed(12345);
        let mut random_source1 = RandomSource::Xoroshiro(rng1);
        let rng2 = Xoroshiro::from_seed(12345);
        let mut random_source2 = RandomSource::Xoroshiro(rng2);

        let noise1 = ImprovedNoise::new(&mut random_source1);
        let noise2 = ImprovedNoise::new(&mut random_source2);

        // Same seed should produce same noise
        assert_eq!(
            noise1.noise(0.5, 0.5, 0.5).to_bits(),
            noise2.noise(0.5, 0.5, 0.5).to_bits()
        );
    }

    #[test]
    fn test_improved_noise_range() {
        let rng = Xoroshiro::from_seed(42);
        let mut random_source = RandomSource::Xoroshiro(rng);
        let noise = ImprovedNoise::new(&mut random_source);

        // Test that values are in expected range [-1, 1]
        for x in 0..10 {
            for y in 0..10 {
                for z in 0..10 {
                    let value =
                        noise.noise(f64::from(x) * 0.1, f64::from(y) * 0.1, f64::from(z) * 0.1);
                    assert!((-1.5..=1.5).contains(&value), "Value out of range: {value}");
                }
            }
        }
    }

    #[test]
    fn test_improved_noise_continuity() {
        let rng = Xoroshiro::from_seed(42);
        let mut random_source = RandomSource::Xoroshiro(rng);
        let noise = ImprovedNoise::new(&mut random_source);

        // Test that noise is continuous (small steps produce small changes)
        let step = 0.001;
        let base = noise.noise(0.5, 0.5, 0.5);
        let nearby = noise.noise(0.5 + step, 0.5, 0.5);
        assert!(
            (base - nearby).abs() < 0.1,
            "Noise not continuous: {base} vs {nearby}"
        );
    }

    /// Tests that our `ImprovedNoise` matches Pumpkin's expected values for vanilla parity.
    /// These values are from Pumpkin's `PerlinNoiseSampler` tests.
    #[test]
    fn test_vanilla_parity_origins() {
        let mut rng = Xoroshiro::from_seed(111);
        // Verify RNG produces expected first value (this consumes one value from the RNG)
        assert_eq!(rng.next_i32(), -1_467_508_761);

        // Create noise from the SAME RNG instance (after next_i32 was called)
        // This matches how Pumpkin's test works
        let mut random_source = RandomSource::Xoroshiro(rng);
        let noise = ImprovedNoise::new(&mut random_source);

        // Expected values from Pumpkin's test_create test
        assert_eq!(noise.xo.to_bits(), 48.580_720_367_179_74_f64.to_bits());
        assert_eq!(noise.yo.to_bits(), 110.732_358_826_780_37_f64.to_bits());
        assert_eq!(noise.zo.to_bits(), 65.264_388_528_601_76_f64.to_bits());
    }

    /// Tests sample values match Pumpkin's expected outputs.
    #[test]
    fn test_vanilla_parity_sample_values() {
        let mut rng = Xoroshiro::from_seed(111);
        // Consume first value to match Pumpkin's test setup
        assert_eq!(rng.next_i32(), -1_467_508_761);

        let mut random_source = RandomSource::Xoroshiro(rng);
        let noise = ImprovedNoise::new(&mut random_source);

        // Expected values from Pumpkin's test_no_y test (sample_flat_y)
        let test_cases: [((f64, f64, f64), f64); 5] = [
            (
                (
                    -3.134_738_528_791_615E8,
                    5.676_610_095_659_718E7,
                    2.011_711_832_498_507E8,
                ),
                0.385_821_396_146_029_45,
            ),
            (
                (
                    -1_369_026.560_586_418,
                    3.957_311_252_810_864E8,
                    6.797_037_355_570_006E8,
                ),
                0.157_775_013_331_571_93,
            ),
            (
                (
                    6.439_373_693_833_767E8,
                    -3.362_187_730_417_59E8,
                    -3.265_494_249_695_775E8,
                ),
                -0.280_613_591_240_949_7,
            ),
            (
                (
                    1.353_820_060_118_252E8,
                    -3.204_701_624_793_043E8,
                    -4.612_474_746_056_331E8,
                ),
                -0.150_528_655_008_377_87,
            ),
            (
                (
                    -6_906_850.625_560_562,
                    1.015_366_394_883_801_3E8,
                    2.492_318_547_830_557_5E8,
                ),
                -0.307_930_069_455_831_8,
            ),
        ];

        for ((x, y, z), expected) in test_cases {
            let result = noise.noise(x, y, z);
            assert_eq!(
                result.to_bits(),
                expected.to_bits(),
                "Mismatch at ({x}, {y}, {z}): got {result}, expected {expected}"
            );
        }
    }
}
