//! Math utilities for noise generation.
//!
//! These functions are exact ports of Minecraft's Mth class methods
//! used in noise generation, ensuring vanilla parity.

/// Smoothstep function using quintic interpolation.
/// Formula: x³(6x² - 15x + 10)
#[inline]
#[must_use]
pub fn smoothstep(x: f64) -> f64 {
    x * x * x * (x * (x * 6.0 - 15.0) + 10.0)
}

/// Derivative of the smoothstep function.
/// Formula: 30x²(x - 1)²
#[inline]
#[must_use]
pub fn smoothstep_derivative(x: f64) -> f64 {
    30.0 * x * x * (x - 1.0) * (x - 1.0)
}

/// Linear interpolation between two values.
#[inline]
#[must_use]
pub fn lerp(delta: f64, start: f64, end: f64) -> f64 {
    start + delta * (end - start)
}

/// Linear interpolation between two f32 values.
#[inline]
#[must_use]
pub fn lerp_f32(delta: f32, start: f32, end: f32) -> f32 {
    start + delta * (end - start)
}

/// Bilinear interpolation.
#[inline]
#[must_use]
pub fn lerp2(delta1: f64, delta2: f64, v00: f64, v10: f64, v01: f64, v11: f64) -> f64 {
    lerp(delta2, lerp(delta1, v00, v10), lerp(delta1, v01, v11))
}

/// Trilinear interpolation.
#[allow(clippy::too_many_arguments)] // Trilinear interpolation inherently needs 3 deltas + 8 corner values
#[inline]
#[must_use]
pub fn lerp3(
    delta1: f64,
    delta2: f64,
    delta3: f64,
    v000: f64,
    v100: f64,
    v010: f64,
    v110: f64,
    v001: f64,
    v101: f64,
    v011: f64,
    v111: f64,
) -> f64 {
    lerp(
        delta3,
        lerp2(delta1, delta2, v000, v100, v010, v110),
        lerp2(delta1, delta2, v001, v101, v011, v111),
    )
}

/// Clamped linear interpolation.
#[inline]
#[must_use]
pub fn clamped_lerp(start: f64, end: f64, delta: f64) -> f64 {
    if delta < 0.0 {
        start
    } else if delta > 1.0 {
        end
    } else {
        lerp(delta, start, end)
    }
}

/// Floor function returning i64 (matches Java's lfloor).
#[inline]
#[must_use]
pub fn lfloor(value: f64) -> i64 {
    let i = value as i64;
    if value < i as f64 { i - 1 } else { i }
}

/// Floor function returning i32 (matches Java's floor).
#[inline]
#[must_use]
pub fn floor(value: f64) -> i32 {
    let i = value as i32;
    if value < f64::from(i) { i - 1 } else { i }
}

/// Clamp a value between min and max.
#[inline]
#[must_use]
pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Floor division matching vanilla Minecraft.
///
/// Returns the largest integer less than or equal to the quotient.
#[inline]
#[must_use]
pub fn floor_div(a: i32, b: i32) -> i32 {
    let q = a / b;
    let r = a % b;
    if r != 0 && (a < 0) != (b < 0) {
        q - 1
    } else {
        q
    }
}

/// Floor modulo matching vanilla Minecraft.
#[inline]
#[must_use]
pub fn floor_mod(a: i32, b: i32) -> i32 {
    let r = a % b;
    if r != 0 && (a < 0) != (b < 0) {
        r + b
    } else {
        r
    }
}

/// Floor modulo for usize values.
#[inline]
#[must_use]
pub fn floor_mod_usize(a: usize, b: usize) -> usize {
    ((a % b) + b) % b
}

/// Maps a value from one range to another without clamping.
#[inline]
#[must_use]
pub fn map(value: f64, old_start: f64, old_end: f64, new_start: f64, new_end: f64) -> f64 {
    let t = (value - old_start) / (old_end - old_start);
    lerp(t, new_start, new_end)
}

/// Maps a value from one range to another, clamping to the output range.
///
/// If value is outside `[old_start, old_end]`, the result is clamped to `[new_start, new_end]`.
#[inline]
#[must_use]
pub fn clamped_map(value: f64, old_start: f64, old_end: f64, new_start: f64, new_end: f64) -> f64 {
    let t = (value - old_start) / (old_end - old_start);
    let clamped_t = t.clamp(0.0, 1.0);
    lerp(clamped_t, new_start, new_end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoothstep() {
        assert_eq!(smoothstep(0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(smoothstep(1.0).to_bits(), 1.0_f64.to_bits());
        assert_eq!(smoothstep(0.5).to_bits(), 0.5_f64.to_bits());
        // Intermediate value
        let result = smoothstep(0.25);
        assert!((result - 0.103_515_625).abs() < 1e-10);
    }

    #[test]
    fn test_lerp() {
        assert_eq!(lerp(0.0, 10.0, 20.0).to_bits(), 10.0_f64.to_bits());
        assert_eq!(lerp(1.0, 10.0, 20.0).to_bits(), 20.0_f64.to_bits());
        assert_eq!(lerp(0.5, 10.0, 20.0).to_bits(), 15.0_f64.to_bits());
    }

    #[test]
    fn test_lfloor() {
        assert_eq!(lfloor(1.5), 1);
        assert_eq!(lfloor(-1.5), -2);
        assert_eq!(lfloor(0.0), 0);
        assert_eq!(lfloor(-0.1), -1);
    }

    #[test]
    fn test_floor() {
        assert_eq!(floor(1.5), 1);
        assert_eq!(floor(-1.5), -2);
        assert_eq!(floor(0.0), 0);
        assert_eq!(floor(-0.1), -1);
    }
}
