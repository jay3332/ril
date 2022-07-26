//! Houses Encoder, Decoder, and frame iterator traits.

use crate::{Error, Frame, Image, ImageSequence, Pixel};
use std::io::{Read, Write};
use std::marker::PhantomData;

#[cfg(feature = "gif")]
use crate::encodings::gif::GifFrameIterator;
#[cfg(feature = "png")]
use crate::encodings::png::ApngFrameIterator;
#[cfg(feature = "webp")]
use crate::encodings::webp::WebPSequenceDecoder;

/// Low-level encoder interface around an image format.
pub trait Encoder {
    /// Encodes the given image into the given writer.
    ///
    /// # Errors
    /// * An error occured during encoding.
    fn encode<P: Pixel>(&mut self, image: &Image<P>, dest: &mut impl Write) -> crate::Result<()>;

    /// Encodes the given image sequence into the given writer.
    ///
    /// # Errors
    /// * An error occured during encoding.
    fn encode_sequence<P: Pixel>(
        &mut self,
        sequence: &ImageSequence<P>,
        dest: &mut impl Write,
    ) -> crate::Result<()> {
        self.encode(
            sequence
                .first_frame()
                .ok_or(Error::EmptyImageError)?
                .image(),
            dest,
        )
    }
}

/// Low-level decoder interface around an image format.
pub trait Decoder<P: Pixel, R: Read> {
    /// The type of the iterator returned by `decode_sequence`.
    type Sequence: FrameIterator<P>;

    /// Decodes the given stream into an image.
    ///
    /// # Errors
    /// * An error occured during decoding.
    fn decode(&mut self, stream: R) -> crate::Result<Image<P>>;

    /// Decodes the given stream into a frame iterator.
    ///
    /// # Errors
    /// * An error occured during decoding.
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

    /// Returns if there are no frames in the sequence. In this case, the image is probably
    /// invalid to be encoded again.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the amount of times this sequence will loop over itself.
    fn loop_count(&self) -> crate::LoopCount;

    /// Collects all frames in this iterator and turns it into a high level [`ImageSequence`].
    /// If any frame fails, that error is returned.
    ///
    /// # Errors
    /// * An error occured during decoding one of the frames.
    fn into_sequence(self) -> crate::Result<ImageSequence<P>>
    where
        Self: Sized,
    {
        let loop_count = self.loop_count();
        let frames = self.collect::<crate::Result<Vec<_>>>()?;

        Ok(ImageSequence::from_frames(frames).with_loop_count(loop_count))
    }
}

/// Represents any one of the different types of frame iterators, compacted into one common enum
/// with common methods.
#[allow(clippy::large_enum_variant)]
pub enum DynamicFrameIterator<P: Pixel, R: Read> {
    /// A single static image frame.
    Single(Option<Image<P>>, PhantomData<R>),
    /// A PNG or APNG frame iterator.
    #[cfg(feature = "png")]
    Png(ApngFrameIterator<P, R>),
    /// A GIF frame iterator.
    #[cfg(feature = "gif")]
    Gif(GifFrameIterator<P, R>),
    /// A WebP frame iterator.
    #[cfg(feature = "webp")]
    WebP(WebPSequenceDecoder<P>),
}

impl<P: Pixel, R: Read> DynamicFrameIterator<P, R> {
    /// Create a new single static image frame iterator.
    #[must_use]
    pub const fn single(image: Image<P>) -> Self {
        Self::Single(Some(image), PhantomData)
    }
}

impl<P: Pixel, R: Read> FrameIterator<P> for DynamicFrameIterator<P, R> {
    fn len(&self) -> u32 {
        match self {
            Self::Single(..) => 1,
            #[cfg(feature = "png")]
            Self::Png(it) => it.len(),
            #[cfg(feature = "gif")]
            Self::Gif(it) => it.len(),
            #[cfg(feature = "webp")]
            Self::WebP(it) => it.len(),
        }
    }

    fn loop_count(&self) -> crate::LoopCount {
        match self {
            Self::Single(..) => crate::LoopCount::Exactly(1),
            #[cfg(feature = "png")]
            Self::Png(it) => it.loop_count(),
            #[cfg(feature = "gif")]
            Self::Gif(it) => it.loop_count(),
            #[cfg(feature = "webp")]
            Self::WebP(it) => it.loop_count(),
        }
    }

    fn into_sequence(self) -> crate::Result<ImageSequence<P>> {
        match self {
            Self::Single(mut it, _) => {
                let image = it.take().unwrap();
                let frame = Frame::from_image(image);

                Ok(ImageSequence::new().with_frame(frame))
            }
            #[cfg(feature = "png")]
            Self::Png(it) => it.into_sequence(),
            #[cfg(feature = "gif")]
            Self::Gif(it) => it.into_sequence(),
            #[cfg(feature = "webp")]
            Self::WebP(it) => it.into_sequence(),
        }
    }
}

impl<P: Pixel, R: Read> Iterator for DynamicFrameIterator<P, R> {
    type Item = crate::Result<Frame<P>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(it, _) => it.take().map(|image| Ok(Frame::from_image(image))),
            #[cfg(feature = "png")]
            Self::Png(it) => it.next(),
            #[cfg(feature = "gif")]
            Self::Gif(it) => it.next(),
            #[cfg(feature = "webp")]
            Self::WebP(it) => it.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Single(Some(_), _) => (1, Some(1)),
            Self::Single(None, _) => (0, Some(0)),
            #[cfg(feature = "png")]
            Self::Png(it) => it.size_hint(),
            #[cfg(feature = "gif")]
            Self::Gif(it) => it.size_hint(),
            #[cfg(feature = "webp")]
            Self::WebP(it) => it.size_hint(),
        }
    }
}
