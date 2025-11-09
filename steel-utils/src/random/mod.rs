use enum_dispatch::enum_dispatch;

use crate::random::xoroshiro::{Xoroshiro, XoroshiroSplitter};

pub mod gaussian;
pub mod xoroshiro;

#[enum_dispatch]
pub trait Random {
    fn fork(&mut self) -> Self;

    fn next_i32(&mut self) -> i32;

    fn next_i32_bounded(&mut self, bound: i32) -> i32;

    fn next_i32_between(&mut self, min: i32, max: i32) -> i32 {
        self.next_i32_bounded(max - min + 1) + min
    }

    fn next_i32_between_exclusive(&mut self, min: i32, max: i32) -> i32 {
        min + self.next_i32_bounded(max - min)
    }

    fn next_i64(&mut self) -> i64;

    fn next_f32(&mut self) -> f32;

    fn next_f64(&mut self) -> f64;

    fn next_bool(&mut self) -> bool;

    fn next_gaussian(&mut self) -> f64;

    fn triangle(&mut self, min: f64, max: f64) -> f64 {
        min + max * (self.next_f64() - self.next_f64())
    }

    fn triangle_f32(&mut self, min: f32, max: f32) -> f32 {
        min + max * (self.next_f32() - self.next_f32())
    }

    fn next_positional(&mut self) -> RandomSplitter;

    fn consume_count(&mut self, count: i32) {
        for _ in 0..count {
            self.next_i64();
        }
    }
}

#[enum_dispatch]
pub trait PositionalRandom {
    fn at(&self, x: i32, y: i32, z: i32) -> RandomSource;

    fn with_hash_of(&self, name: &str) -> RandomSource;

    fn with_seed(&self, seed: u64) -> RandomSource;
}

#[enum_dispatch(Random)]
pub enum RandomSource {
    Xoroshiro(Xoroshiro),
}

#[enum_dispatch(PositionalRandom)]
pub enum RandomSplitter {
    Xoroshiro(XoroshiroSplitter),
}

pub fn get_seed(x: i32, y: i32, z: i32) -> i64 {
    let l = (x.wrapping_mul(3129871) as i64) ^ ((z as i64).wrapping_mul(116129781i64)) ^ (y as i64);
    let l = l
        .wrapping_mul(l)
        .wrapping_mul(42317861i64)
        .wrapping_add(l.wrapping_mul(11i64));
    l >> 16
}
