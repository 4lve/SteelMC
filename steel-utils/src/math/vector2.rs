use std::{
    io::{Read, Result, Write},
    ops::{Add, Div, Mul, Neg, Sub},
};

use crate::serial::{ReadFrom, WriteTo};

#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq, Default)]
pub struct Vector2<T> {
    pub x: T,
    pub y: T,
}

impl<T: Math + Copy> Vector2<T> {
    pub const fn new(x: T, y: T) -> Self {
        Vector2 { x, y }
    }

    pub fn length_squared(&self) -> T {
        self.x * self.x + self.y * self.y
    }

    pub fn add(&self, other: &Vector2<T>) -> Self {
        Vector2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    pub fn add_raw(&self, x: T, y: T) -> Self {
        Vector2 {
            x: self.x + x,
            y: self.y + y,
        }
    }

    pub fn sub(&self, other: &Vector2<T>) -> Self {
        Vector2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }

    pub fn multiply(self, x: T, y: T) -> Self {
        Self {
            x: self.x * x,
            y: self.y * y,
        }
    }
}

impl<T: WriteTo> WriteTo for Vector2<T> {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.x.write(writer)?;
        self.y.write(writer)
    }
}

impl<T: ReadFrom> ReadFrom for Vector2<T> {
    fn read(data: &mut impl Read) -> Result<Self> {
        Ok(Self {
            x: T::read(data)?,
            y: T::read(data)?,
        })
    }
}

pub trait Math:
    Mul<Output = Self>
    + Neg<Output = Self>
    + Add<Output = Self>
    + Div<Output = Self>
    + Sub<Output = Self>
    + Sized
{
}
impl Math for f64 {}
impl Math for f32 {}
impl Math for i32 {}
impl Math for i64 {}
impl Math for i8 {}
