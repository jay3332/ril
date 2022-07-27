use crate::Error::DecodingError;
use crate::{Image, Pixel};

pub struct ByteStream<'buf> {
    data: &'buf [u8],
    position: usize,
}

impl<'buf> ByteStream<'buf> {
    #[must_use]
    pub const fn new(data: &'buf [u8]) -> Self {
        Self { data, position: 0 }
    }

    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }

    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.data.len() - self.position
    }

    pub fn read_to_end(&mut self) -> &[u8] {
        let data = &self.data[self.position..];
        self.position = self.data.len();

        data
    }

    pub fn peek(&self, bytes: usize) -> &[u8] {
        if bytes > self.remaining() {
            return &self.data[self.position..];
        }

        &self.data[self.position..self.position + bytes]
    }

    pub fn read(&mut self, bytes: usize) -> &[u8] {
        if bytes == 0 {
            return &[];
        }

        if self.remaining() < bytes {
            return self.read_to_end();
        }

        let start = self.position;
        self.position += bytes;

        &self.data[start..self.position]
    }

    /// Reads data and transmutes it to T.
    /// 
    /// # Panics
    /// * There is not enough data to read
    pub fn read_to<T>(&mut self) -> T {
        let size = std::mem::size_of::<T>();
        let data = self.read(size);

        assert_eq!(data.len(), size, "Not enough data to read");

        // SAFETY: we check if the data is the same length as T above
        unsafe { (data as *const _ as *const T).read() }
    }

    pub fn read_u8(&mut self) -> crate::Result<u8> {
        self.read(1)
            .first()
            .copied()
            .ok_or(DecodingError("Expected 1 more byte to convert into u8"))
    }

    pub fn read_u32(&mut self) -> crate::Result<u32> {
        Ok(u32::from_be_bytes(self.read(4).try_into().map_err(
            |_| DecodingError("Expected 4 bytes to convert into u32"),
        )?))
    }

    pub fn rewind(&mut self, bytes: usize) {
        self.position -= bytes;
    }

    pub fn seek(&mut self, offset: usize) {
        self.position = offset;
    }
}

pub trait Decoder {
    fn decode<P: Pixel>(&mut self, stream: &mut ByteStream) -> crate::Result<Image<P>>;
}
