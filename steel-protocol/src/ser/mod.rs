use std::io::{Read, Write};

use crate::codec::{
    errors::{ReadingError, WritingError},
    var_int, var_uint,
};

pub trait NetworkReadExt {
    fn get_i8(&mut self) -> Result<i8, ReadingError>;
    fn get_u8(&mut self) -> Result<u8, ReadingError>;

    fn get_i16_be(&mut self) -> Result<i16, ReadingError>;
    fn get_u16_be(&mut self) -> Result<u16, ReadingError>;
    fn get_i32_be(&mut self) -> Result<i32, ReadingError>;
    fn get_u32_be(&mut self) -> Result<u32, ReadingError>;
    fn get_i64_be(&mut self) -> Result<i64, ReadingError>;
    fn get_u64_be(&mut self) -> Result<u64, ReadingError>;
    fn get_f32_be(&mut self) -> Result<f32, ReadingError>;
    fn get_f64_be(&mut self) -> Result<f64, ReadingError>;
    fn get_i128_be(&mut self) -> Result<i128, ReadingError>;
    fn get_u128_be(&mut self) -> Result<u128, ReadingError>;
    fn read_boxed_slice(&mut self, count: usize) -> Result<Box<[u8]>, ReadingError>;

    fn read_remaining_to_boxed_slice(&mut self, bound: usize) -> Result<Box<[u8]>, ReadingError>;

    fn get_bool(&mut self) -> Result<bool, ReadingError>;
    fn get_var_int(&mut self) -> Result<i32, ReadingError>;
    fn get_var_uint(&mut self) -> Result<u32, ReadingError>;

    fn get_string_bounded(&mut self, bound: usize) -> Result<String, ReadingError>;
    fn get_string(&mut self) -> Result<String, ReadingError>;
}

macro_rules! get_number_be {
    ($name:ident, $type:ty) => {
        fn $name(&mut self) -> Result<$type, ReadingError> {
            let mut buf = [0u8; std::mem::size_of::<$type>()];
            self.read_exact(&mut buf)
                .map_err(|err| ReadingError::Incomplete(err.to_string()))?;
            Ok(<$type>::from_be_bytes(buf))
        }
    };
}

impl<R: Read> NetworkReadExt for R {
    //TODO: Macroize this
    fn get_i8(&mut self) -> Result<i8, ReadingError> {
        let mut buf = [0u8];
        self.read_exact(&mut buf)
            .map_err(|err| ReadingError::Incomplete(err.to_string()))?;

        Ok(buf[0] as i8)
    }

    fn get_u8(&mut self) -> Result<u8, ReadingError> {
        let mut buf = [0u8];
        self.read_exact(&mut buf)
            .map_err(|err| ReadingError::Incomplete(err.to_string()))?;

        Ok(buf[0])
    }

    get_number_be!(get_i16_be, i16);
    get_number_be!(get_u16_be, u16);
    get_number_be!(get_i32_be, i32);
    get_number_be!(get_u32_be, u32);
    get_number_be!(get_i64_be, i64);
    get_number_be!(get_u64_be, u64);
    get_number_be!(get_i128_be, i128);
    get_number_be!(get_u128_be, u128);
    get_number_be!(get_f32_be, f32);
    get_number_be!(get_f64_be, f64);

    fn read_boxed_slice(&mut self, count: usize) -> Result<Box<[u8]>, ReadingError> {
        let mut buf = vec![0u8; count];
        self.read_exact(&mut buf)
            .map_err(|err| ReadingError::Incomplete(err.to_string()))?;

        Ok(buf.into())
    }

    fn read_remaining_to_boxed_slice(&mut self, bound: usize) -> Result<Box<[u8]>, ReadingError> {
        let mut return_buf = Vec::new();

        // TODO: We can probably remove the temp buffer somehow
        let mut temp_buf = [0; 1024];
        loop {
            let bytes_read = self
                .read(&mut temp_buf)
                .map_err(|err| ReadingError::Incomplete(err.to_string()))?;

            if bytes_read == 0 {
                break;
            }

            if return_buf.len() + bytes_read > bound {
                return Err(ReadingError::TooLarge(
                    "Read remaining too long".to_string(),
                ));
            }

            return_buf.extend(&temp_buf[..bytes_read]);
        }
        Ok(return_buf.into_boxed_slice())
    }

    fn get_bool(&mut self) -> Result<bool, ReadingError> {
        let byte = self.get_u8()?;
        Ok(byte != 0)
    }

    fn get_var_int(&mut self) -> Result<i32, ReadingError> {
        var_int::read(self)
    }

    fn get_var_uint(&mut self) -> Result<u32, ReadingError> {
        var_uint::read(self)
    }

    fn get_string_bounded(&mut self, bound: usize) -> Result<String, ReadingError> {
        let size = self.get_var_uint()? as usize;
        if size > bound {
            return Err(ReadingError::TooLarge("string".to_string()));
        }

        let data = self.read_boxed_slice(size)?;
        String::from_utf8(data.into()).map_err(|e| ReadingError::Message(e.to_string()))
    }

    fn get_string(&mut self) -> Result<String, ReadingError> {
        self.get_string_bounded(i32::MAX as usize)
    }
}

pub trait NetworkWriteExt {
    fn write_i8(&mut self, data: i8) -> Result<(), WritingError>;
    fn write_u8(&mut self, data: u8) -> Result<(), WritingError>;
    fn write_i16_be(&mut self, data: i16) -> Result<(), WritingError>;
    fn write_u16_be(&mut self, data: u16) -> Result<(), WritingError>;
    fn write_i32_be(&mut self, data: i32) -> Result<(), WritingError>;
    fn write_u32_be(&mut self, data: u32) -> Result<(), WritingError>;
    fn write_i64_be(&mut self, data: i64) -> Result<(), WritingError>;
    fn write_u64_be(&mut self, data: u64) -> Result<(), WritingError>;
    fn write_f32_be(&mut self, data: f32) -> Result<(), WritingError>;
    fn write_f64_be(&mut self, data: f64) -> Result<(), WritingError>;
    fn write_slice(&mut self, data: &[u8]) -> Result<(), WritingError>;

    fn write_bool(&mut self, data: bool) -> Result<(), WritingError> {
        if data {
            self.write_u8(1)
        } else {
            self.write_u8(0)
        }
    }

    fn write_var_int(&mut self, data: i32) -> Result<(), WritingError>;
    fn write_var_uint(&mut self, data: u32) -> Result<(), WritingError>;

    fn write_string_bounded(&mut self, data: &str, bound: usize) -> Result<(), WritingError>;
    fn write_string(&mut self, data: &str) -> Result<(), WritingError>;
}

macro_rules! write_number_be {
    ($name:ident, $type:ty) => {
        fn $name(&mut self, data: $type) -> Result<(), WritingError> {
            self.write_all(&data.to_be_bytes())
                .map_err(WritingError::IoError)
        }
    };
}

impl<W: Write> NetworkWriteExt for W {
    fn write_i8(&mut self, data: i8) -> Result<(), WritingError> {
        self.write_all(&data.to_be_bytes())
            .map_err(WritingError::IoError)
    }

    fn write_u8(&mut self, data: u8) -> Result<(), WritingError> {
        self.write_all(&data.to_be_bytes())
            .map_err(WritingError::IoError)
    }

    write_number_be!(write_i16_be, i16);
    write_number_be!(write_u16_be, u16);
    write_number_be!(write_i32_be, i32);
    write_number_be!(write_u32_be, u32);
    write_number_be!(write_i64_be, i64);
    write_number_be!(write_u64_be, u64);
    write_number_be!(write_f32_be, f32);
    write_number_be!(write_f64_be, f64);

    fn write_slice(&mut self, data: &[u8]) -> Result<(), WritingError> {
        self.write_all(data).map_err(WritingError::IoError)
    }

    fn write_var_int(&mut self, data: i32) -> Result<(), WritingError> {
        var_int::write(data, self)
    }

    fn write_var_uint(&mut self, data: u32) -> Result<(), WritingError> {
        var_uint::write(&data, self)
    }

    fn write_string_bounded(&mut self, data: &str, bound: usize) -> Result<(), WritingError> {
        assert!(data.len() <= bound);
        self.write_var_int(data.len().try_into().map_err(|_| {
            WritingError::Message(format!("{} isn't representable as a VarInt", data.len()))
        })?)?;

        self.write_all(data.as_bytes())
            .map_err(WritingError::IoError)
    }

    fn write_string(&mut self, data: &str) -> Result<(), WritingError> {
        self.write_string_bounded(data, i16::MAX as usize)
    }
}
