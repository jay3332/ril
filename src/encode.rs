use crate::{encodings::png::ApngFrameIterator, Frame, Image, ImageSequence, Pixel};
use std::io::{Read, Write};

pub trait Encoder {
    fn encode<P: Pixel>(&mut self, image: &Image<P>, dest: &mut impl Write) -> crate::Result<()>;

    fn encode_sequence<P: Pixel>(
        &mut self,
        sequence: &ImageSequence<P>,
        dest: &mut impl Write,
    ) -> crate::Result<()> {
        self.encode(sequence.first_frame().image(), dest)
    }
}

pub trait Decoder<P: Pixel, R: Read> {
    type Sequence: FrameIterator<P>;

    fn decode(&mut self, stream: R) -> crate::Result<Image<P>>;

    fn decode_sequence(&mut self, stream: R) -> crate::Result<Self::Sequence>;
}

/// Represents the lazy decoding of frames from an encoded image sequence, such as an animated
/// image.
///
/// # See Also
/// * [`ImageSequence`]
/// * [`Frame`]
pub trait FrameIterator<P: Pixel>: Iterator<Item = crate::Result<Frame<P>>> {
    /// Returns the number of frames in the sequence.
    ///
    /// This does not consume any frames as this data is usually known from the very beginning of
    /// decoding.
    fn len(&self) -> u32;

    /// Returns the amount of times this sequence will loop over itself.
    fn loop_count(&self) -> crate::LoopCount;

    /// Collects all frames in this iterator and turns it into a high level [`ImageSequence`].
    ///
    /// If any frame fails, that error is returned.
    fn into_sequence(self) -> crate::Result<ImageSequence<P>>;
}

/// Represents any one of the different types of frame iterators, compacted into one common enum
/// with common methods.
pub enum DynamicFrameIterator<P: Pixel, R: Read> {
    /// A PNG or APNG frame iterator.
    Png(ApngFrameIterator<P, R>),
}

impl<P: Pixel, R: Read> FrameIterator<P> for DynamicFrameIterator<P, R> {
    fn len(&self) -> u32 {
        match self {
            DynamicFrameIterator::Png(it) => it.len(),
        }
    }

    fn loop_count(&self) -> crate::LoopCount {
        match self {
            DynamicFrameIterator::Png(it) => it.loop_count(),
        }
    }

    fn into_sequence(self) -> crate::Result<ImageSequence<P>> {
        match self {
            DynamicFrameIterator::Png(it) => it.into_sequence(),
        }
    }
}

impl<P: Pixel, R: Read> Iterator for DynamicFrameIterator<P, R> {
    type Item = crate::Result<Frame<P>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            DynamicFrameIterator::Png(it) => it.next(),
        }
    }
}
