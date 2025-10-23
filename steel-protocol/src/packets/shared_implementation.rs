use std::io;

use steel_utils::text::TextComponentBase;

use crate::packet_traits::Write;

impl Write for TextComponentBase {
    fn write(&self, _: &mut impl io::Write) -> Result<(), io::Error> {
        //TODO: Implement
        todo!()
    }
}
