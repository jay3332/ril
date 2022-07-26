//! Simple wrapper around miniz_oxide for inflating data.
//!
//! This was created with https://github.com/image-rs/image-png/blob/master/src/decoder/zlib.rs
//! as a reference.

use crate::{Error::DecodingError, Result};
use miniz_oxide::inflate::{
    core::{decompress, inflate_flags, DecompressorOxide},
    TINFLStatus,
};

pub const BUFFER_SIZE: usize = 32768;

pub struct ZlibReader {
    inner: Box<DecompressorOxide>,
    started: bool,
    buffer: Vec<u8>,
    buffer_start: usize,
    output_buffer: Vec<u8>,
    output_buffer_start: usize,
}

impl ZlibReader {
    pub fn new() -> Self {
        Self {
            inner: Box::new(DecompressorOxide::default()),
            started: false,
            buffer: Vec::with_capacity(BUFFER_SIZE),
            buffer_start: 0,
            output_buffer: vec![0; BUFFER_SIZE * 2],
            output_buffer_start: 0,
        }
    }

    pub fn reset(&mut self) {
        self.started = false;
        self.buffer.clear();
        self.output_buffer.clear();
        self.output_buffer_start = 0;
        *self.inner = DecompressorOxide::default();
    }

    pub fn prepare(&mut self) {
        if self
            .output_buffer
            .len()
            .saturating_sub(self.output_buffer_start)
            >= BUFFER_SIZE
        {
            return;
        }

        let len = self.output_buffer.len();

        let len = len
            .saturating_add(BUFFER_SIZE.max(len))
            .min(u64::MAX as usize)
            .min(isize::MAX as usize);

        self.output_buffer.resize(len, 0);
    }

    pub fn decompress(&mut self, data: &[u8], dest: &mut Vec<u8>) -> Result<usize> {
        self.prepare();

        let (status, mut consumed, out_consumed) = {
            let data = if self.buffer.is_empty() {
                data
            } else {
                &data[self.buffer_start..]
            };

            decompress(
                &mut self.inner,
                data,
                self.output_buffer.as_mut_slice(),
                self.output_buffer_start,
                inflate_flags::TINFL_FLAG_PARSE_ZLIB_HEADER
                    | inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF
                    | inflate_flags::TINFL_FLAG_HAS_MORE_INPUT,
            )
        };

        self.started = true;
        self.output_buffer_start += out_consumed;

        if !self.buffer.is_empty() {
            self.buffer_start += consumed;
        }

        if self.buffer.len() == self.buffer_start {
            self.buffer.clear();
            self.buffer_start = 0;
        }

        if consumed == 0 {
            self.buffer.extend_from_slice(data);
            consumed = data.len();
        }

        self.write_data(dest);

        match status {
            TINFLStatus::Done | TINFLStatus::NeedsMoreInput | TINFLStatus::HasMoreOutput => {
                Ok(consumed)
            }
            _ => Err(DecodingError("Received corrupt zlib data")),
        }
    }

    pub fn finish(&mut self, dest: &mut Vec<u8>) -> Result<()> {
        if !self.started {
            return Ok(());
        }

        let tail = &self.buffer.split_off(0)[self.buffer_start..];

        let mut start = 0;
        loop {
            self.prepare();

            let (status, consumed, out_consumed) = {
                decompress(
                    &mut self.inner,
                    &tail[start..],
                    self.output_buffer.as_mut_slice(),
                    self.output_buffer_start,
                    inflate_flags::TINFL_FLAG_PARSE_ZLIB_HEADER
                        | inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
                )
            };

            start += consumed;
            self.output_buffer_start += out_consumed;

            match status {
                TINFLStatus::Done => {
                    self.output_buffer.truncate(self.output_buffer_start);
                    dest.append(&mut self.output_buffer);

                    return Ok(());
                }
                TINFLStatus::HasMoreOutput => {
                    self.write_data(dest);
                }
                _ => return Err(DecodingError("Received corrupt zlib data")),
            }
        }
    }

    fn write_data(&mut self, data: &mut Vec<u8>) -> usize {
        let safe = self.output_buffer_start.saturating_sub(BUFFER_SIZE);

        data.extend(self.output_buffer.drain(..safe));
        self.output_buffer_start -= safe;

        safe
    }
}
