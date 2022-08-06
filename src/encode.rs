use crate::{Image, Pixel};
use std::io::{Read, Write};

pub trait Encoder {
    fn encode<P: Pixel>(&mut self, image: &Image<P>, dest: &mut impl Write) -> crate::Result<()>;
}

pub trait Decoder {
    /// todo!()
    ///
    /// # Errors
    /// * todo!()
    fn decode<P: Pixel>(&mut self, stream: impl Read) -> crate::Result<Image<P>>;
}
