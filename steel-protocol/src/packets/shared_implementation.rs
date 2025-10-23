use std::io::{self, Write};

use steel_utils::text::TextComponentBase;

use crate::packet_traits::WriteTo;

impl WriteTo for TextComponentBase {
    fn write(&self, _: &mut impl Write) -> Result<(), io::Error> {
        //TODO: Implement
        todo!()
    }
}
