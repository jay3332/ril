use crate::{Image, Pixel};

pub struct ByteStream<'buf> {
    data: &'buf [u8],
    position: usize,
}

impl<'buf> ByteStream<'buf> {
    pub fn new(data: &[u8]) -> Self {
        Self {
            data,
            position: 0,
        }
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.position
    }

    pub fn read_to_end(&mut self) -> &[u8] {
        let data = &self.data[self.position..];
        self.position = self.data.len();

        data
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

    pub fn read_to<T>(&mut self) -> T {
        let size = std::mem::size_of::<T>();
        let data = self.read(size);

        if data.len() != size {
            panic!("Not enough data to read");
        }
        
        // SAFETY: we check if the data is the same length as T above
        unsafe { (data as *const _ as *const T).read() }
    }

    pub fn rewind(&mut self, bytes: usize) {
        self.position -= bytes;
    }

    pub fn seek(&mut self, offset: usize) {
        self.position = offset;
    }
}

pub trait Decoder<P: Pixel> {
    fn decode(&mut self, stream: &mut ByteStream) -> Image<P>;
}
