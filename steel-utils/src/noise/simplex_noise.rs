//! Simplex noise implementation for vanilla-accurate world generation.
//!
//! This is an exact port of Minecraft's `SimplexNoise` class.

// Noise code uses mathematical single-letter variables (x, y, z, i, j, k)
#![allow(clippy::many_single_char_names)]

use crate::random::Random;

use super::floor;

/// Gradient vectors for noise computation.
/// These are shared with `ImprovedNoise` for the `grad_dot` function.
pub static GRADIENT: [[i32; 3]; 16] = [
    [1, 1, 0],
    [-1, 1, 0],
    [1, -1, 0],
    [-1, -1, 0],
    [1, 0, 1],
    [-1, 0, 1],
    [1, 0, -1],
    [-1, 0, -1],
    [0, 1, 1],
    [0, -1, 1],
    [0, 1, -1],
    [0, -1, -1],
    [1, 1, 0],
    [0, -1, 1],
    [-1, 1, 0],
    [0, -1, -1],
];

/// Compute dot product of gradient with offset.
#[inline]
pub fn dot(gradient: [i32; 3], x: f64, y: f64, z: f64) -> f64 {
    f64::from(gradient[0]) * x + f64::from(gradient[1]) * y + f64::from(gradient[2]) * z
}

/// Simplex noise generator.
///
/// Implements 2D and 3D simplex noise matching Minecraft's implementation exactly.
pub struct SimplexNoise {
    /// Permutation table (512 entries for wraparound).
    p: [i32; 512],
    /// X offset for noise variation.
    pub xo: f64,
    /// Y offset for noise variation.
    pub yo: f64,
    /// Z offset for noise variation.
    pub zo: f64,
}

impl SimplexNoise {
    // Skew constants for 2D simplex
    // sqrt(3) = 1.732_050_807_568_877_2
    const F2: f64 = 0.366_025_403_784_438_6; // 0.5 * (sqrt(3) - 1)
    const G2: f64 = 0.211_324_865_405_187_1; // (3 - sqrt(3)) / 6

    /// Creates a new `SimplexNoise` from a random source.
    pub fn new<R: Random>(random: &mut R) -> Self {
        let xo = random.next_f64() * 256.0;
        let yo = random.next_f64() * 256.0;
        let zo = random.next_f64() * 256.0;

        let mut p = [0i32; 512];

        // Initialize with identity
        for (i, item) in p[..256].iter_mut().enumerate() {
            *item = i as i32;
        }

        // Fisher-Yates shuffle
        for i in 0..256 {
            let j = random.next_i32_bounded((256 - i) as i32) as usize;
            p.swap(i, i + j);
        }

        // Copy to second half for wraparound (implicit in p() function)

        Self { p, xo, yo, zo }
    }

    /// Get permutation value with wrapping.
    #[inline]
    fn p(&self, index: i32) -> i32 {
        self.p[(index & 255) as usize]
    }

    /// Compute corner contribution for simplex noise.
    #[inline]
    fn corner_noise_3d(gradient_index: i32, x: f64, y: f64, z: f64, offset: f64) -> f64 {
        let d = offset - x * x - y * y - z * z;
        if d < 0.0 {
            0.0
        } else {
            let d = d * d;
            d * d * dot(GRADIENT[(gradient_index & 15) as usize], x, y, z)
        }
    }

    /// Sample 2D simplex noise at the given coordinates.
    #[must_use]
    pub fn get_value_2d(&self, x: f64, y: f64) -> f64 {
        // Skew input space to determine which simplex cell we're in
        let s = (x + y) * Self::F2;
        let i = floor(x + s);
        let j = floor(y + s);

        // Unskew to get simplex origin
        let t = f64::from(i + j) * Self::G2;
        let x0 = f64::from(i) - t;
        let y0 = f64::from(j) - t;

        // Offsets from simplex origin
        let dx0 = x - x0;
        let dy0 = y - y0;

        // Determine which simplex triangle we're in
        let (i1, j1) = if dx0 > dy0 { (1, 0) } else { (0, 1) };

        // Offsets for middle corner
        let dx1 = dx0 - f64::from(i1) + Self::G2;
        let dy1 = dy0 - f64::from(j1) + Self::G2;

        // Offsets for last corner
        let dx2 = dx0 - 1.0 + 2.0 * Self::G2;
        let dy2 = dy0 - 1.0 + 2.0 * Self::G2;

        // Hash coordinates to get gradient indices
        let ii = i & 255;
        let jj = j & 255;
        let gi0 = self.p(ii + self.p(jj)) % 12;
        let gi1 = self.p(ii + i1 + self.p(jj + j1)) % 12;
        let gi2 = self.p(ii + 1 + self.p(jj + 1)) % 12;

        // Calculate contributions from three corners
        let n0 = Self::corner_noise_3d(gi0, dx0, dy0, 0.0, 0.5);
        let n1 = Self::corner_noise_3d(gi1, dx1, dy1, 0.0, 0.5);
        let n2 = Self::corner_noise_3d(gi2, dx2, dy2, 0.0, 0.5);

        // Scale to [-1, 1]
        70.0 * (n0 + n1 + n2)
    }

    /// Sample 3D simplex noise at the given coordinates.
    #[must_use]
    pub fn get_value_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        const F3: f64 = 1.0 / 3.0;
        const G3: f64 = 1.0 / 6.0;

        // Skew input space
        let s = (x + y + z) * F3;
        let i = floor(x + s);
        let j = floor(y + s);
        let k = floor(z + s);

        // Unskew
        let t = f64::from(i + j + k) * G3;
        let x0 = f64::from(i) - t;
        let y0 = f64::from(j) - t;
        let z0 = f64::from(k) - t;

        // Offsets from simplex origin
        let dx0 = x - x0;
        let dy0 = y - y0;
        let dz0 = z - z0;

        // Determine which simplex we're in
        let (i1, j1, k1, i2, j2, k2) = if dx0 >= dy0 {
            if dy0 >= dz0 {
                (1, 0, 0, 1, 1, 0)
            } else if dx0 >= dz0 {
                (1, 0, 0, 1, 0, 1)
            } else {
                (0, 0, 1, 1, 0, 1)
            }
        } else if dy0 < dz0 {
            (0, 0, 1, 0, 1, 1)
        } else if dx0 < dz0 {
            (0, 1, 0, 0, 1, 1)
        } else {
            (0, 1, 0, 1, 1, 0)
        };

        // Offsets for corners
        let dx1 = dx0 - f64::from(i1) + G3;
        let dy1 = dy0 - f64::from(j1) + G3;
        let dz1 = dz0 - f64::from(k1) + G3;

        let dx2 = dx0 - f64::from(i2) + 2.0 * G3;
        let dy2 = dy0 - f64::from(j2) + 2.0 * G3;
        let dz2 = dz0 - f64::from(k2) + 2.0 * G3;

        let dx3 = dx0 - 1.0 + 0.5;
        let dy3 = dy0 - 1.0 + 0.5;
        let dz3 = dz0 - 1.0 + 0.5;

        // Hash coordinates
        let ii = i & 255;
        let jj = j & 255;
        let kk = k & 255;

        let gi0 = self.p(ii + self.p(jj + self.p(kk))) % 12;
        let gi1 = self.p(ii + i1 + self.p(jj + j1 + self.p(kk + k1))) % 12;
        let gi2 = self.p(ii + i2 + self.p(jj + j2 + self.p(kk + k2))) % 12;
        let gi3 = self.p(ii + 1 + self.p(jj + 1 + self.p(kk + 1))) % 12;

        // Calculate contributions
        let n0 = Self::corner_noise_3d(gi0, dx0, dy0, dz0, 0.6);
        let n1 = Self::corner_noise_3d(gi1, dx1, dy1, dz1, 0.6);
        let n2 = Self::corner_noise_3d(gi2, dx2, dy2, dz2, 0.6);
        let n3 = Self::corner_noise_3d(gi3, dx3, dy3, dz3, 0.6);

        // Scale to [-1, 1]
        32.0 * (n0 + n1 + n2 + n3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::xoroshiro::Xoroshiro;

    #[test]
    fn test_simplex_deterministic() {
        let mut rng1 = Xoroshiro::from_seed(12345);
        let mut rng2 = Xoroshiro::from_seed(12345);

        let noise1 = SimplexNoise::new(&mut rng1);
        let noise2 = SimplexNoise::new(&mut rng2);

        // Same seed should produce same noise
        assert_eq!(
            noise1.get_value_2d(0.5, 0.5).to_bits(),
            noise2.get_value_2d(0.5, 0.5).to_bits()
        );
        assert_eq!(
            noise1.get_value_3d(0.5, 0.5, 0.5).to_bits(),
            noise2.get_value_3d(0.5, 0.5, 0.5).to_bits()
        );
    }

    #[test]
    fn test_simplex_range() {
        let mut rng = Xoroshiro::from_seed(42);
        let noise = SimplexNoise::new(&mut rng);

        // Test that values are roughly in expected range
        for x in 0..10 {
            for y in 0..10 {
                let value = noise.get_value_2d(f64::from(x) * 0.1, f64::from(y) * 0.1);
                assert!(
                    (-1.5..=1.5).contains(&value),
                    "2D value out of range: {value}"
                );
            }
        }

        for x in 0..10 {
            for y in 0..10 {
                for z in 0..10 {
                    let value = noise.get_value_3d(
                        f64::from(x) * 0.1,
                        f64::from(y) * 0.1,
                        f64::from(z) * 0.1,
                    );
                    assert!(
                        (-1.5..=1.5).contains(&value),
                        "3D value out of range: {value}"
                    );
                }
            }
        }
    }
}
