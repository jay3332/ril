//! Houses Encoder, Decoder, and frame iterator traits.

use crate::{ColorType, DisposalMethod, Error, Frame, Image, ImageSequence, LoopCount, Pixel};
use std::ops::DerefMut;
use std::{
    io::{Read, Write},
    ops::Deref,
    time::Duration,
};

mod sealed {
    use super::{ColorType, DisposalMethod, Duration, Frame, Image, LoopCount, Pixel};

    pub trait HasEncoderMetadata<C: Default, P: Pixel>: Sized {
        fn width(&self) -> u32;
        fn height(&self) -> u32;
        /// returns `Some((sequence.len(), loop_count))`
        fn sequence(&self) -> Option<(usize, LoopCount)> {
            None
        }
        fn color_type(&self) -> ColorType;
        fn bit_depth(&self) -> u8;
        fn palette(&self) -> Option<&[P::Color]> {
            None
        }
        fn config(self) -> C {
            C::default()
        }
    }

    pub trait FrameLike<P: Pixel> {
        fn image(&self) -> &Image<P>;
        fn delay(&self) -> Option<Duration>;
        fn disposal(&self) -> Option<DisposalMethod>;
    }

    impl<P: Pixel> FrameLike<P> for Image<P> {
        fn image(&self) -> &Self {
            self
        }
        fn delay(&self) -> Option<Duration> {
            None
        }
        fn disposal(&self) -> Option<DisposalMethod> {
            None
        }
    }

    impl<P: Pixel> FrameLike<P> for Frame<P> {
        fn image(&self) -> &Image<P> {
            self.image()
        }
        fn delay(&self) -> Option<Duration> {
            Some(self.delay())
        }
        fn disposal(&self) -> Option<DisposalMethod> {
            Some(self.disposal())
        }
    }
}

pub(crate) use sealed::*;

impl<'a, C: Default, P: Pixel> HasEncoderMetadata<C, P> for &'a Image<P> {
    fn width(&self) -> u32 {
        self.width.get()
    }
    fn height(&self) -> u32 {
        self.height.get()
    }
    fn sequence(&self) -> Option<(usize, LoopCount)> {
        None
    }
    fn color_type(&self) -> ColorType {
        self.data.first().map_or(P::COLOR_TYPE, P::color_type)
    }
    fn bit_depth(&self) -> u8 {
        P::BIT_DEPTH
    }
    fn palette(&self) -> Option<&[P::Color]> {
        Image::palette(self)
    }
}

impl<'a, C: Default, P: Pixel> HasEncoderMetadata<C, P> for &'a Frame<P> {
    fn width(&self) -> u32 {
        self.image().width.get()
    }
    fn height(&self) -> u32 {
        self.image().height.get()
    }
    fn sequence(&self) -> Option<(usize, LoopCount)> {
        Some((1, LoopCount::Infinite))
    }
    fn color_type(&self) -> ColorType {
        self.data.first().map_or(P::COLOR_TYPE, P::color_type)
    }
    fn bit_depth(&self) -> u8 {
        P::BIT_DEPTH
    }
    fn palette(&self) -> Option<&[P::Color]> {
        Image::palette(self.image())
    }
}

impl<'a, C: Default, P: Pixel> HasEncoderMetadata<C, P> for &'a ImageSequence<P> {
    fn width(&self) -> u32 {
        self.first_frame()
            .ok_or(Error::EmptyImageError)
            .unwrap()
            .width
            .get()
    }

    fn height(&self) -> u32 {
        self.first_frame()
            .ok_or(Error::EmptyImageError)
            .unwrap()
            .height
            .get()
    }

    fn sequence(&self) -> Option<(usize, LoopCount)> {
        Some((self.len(), self.loop_count()))
    }

    fn color_type(&self) -> ColorType {
        self.first_frame().map_or(P::COLOR_TYPE, |image| {
            image.data.first().map_or(P::COLOR_TYPE, P::color_type)
        })
    }

    fn bit_depth(&self) -> u8 {
        P::BIT_DEPTH
    }

    fn palette(&self) -> Option<&[P::Color]> {
        self.first_frame()
            .and_then(|frame| Image::palette(frame.image()))
    }
}

/// Manually configured encoder metadata. This is used to provide fine-grained control over the
/// encoder. Not all encoders will use/consider all of these options.
///
/// # See Also
/// - [`Encoder::prepare`] for more information on how to use this.
/// - [`Encoder::encode_static`] to encode a static [`Image`].
/// - [`Encoder::encode_sequence`] to encode an [`ImageSequence`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EncoderMetadata<P: Pixel> {
    /// The width of the image.
    pub width: u32,
    /// The height of the image.
    pub height: u32,
    /// Information regarding whether the image is static (not animated) or not.
    /// * If `Some`, the image is animated and holds `(frame_count, loop_count)`.
    /// * If `None`, the image is considered static.
    ///
    /// # Note
    /// If you do not know the frame count or loop count, substitute `0` for the frame count and
    /// [`LoopCount::Infinite`] for the loop count. All encoders should be able to "grow" this
    /// information as they encode more frames. (This is the [`default`][Default::default] value.)
    pub sequence: Option<(usize, LoopCount)>,
    /// The color type of the image.
    pub color_type: ColorType,
    /// The bit depth of the pixels in the image.
    pub bit_depth: u8,
    /// The palette of the image.
    pub palette: Option<Box<[P::Color]>>,
}

macro_rules! impl_from_metadata {
    ($($t:ident)+) => {
        $(
            impl<'a, P: Pixel> From<&'a $t<P>> for EncoderMetadata<P> {
                fn from(metadata: &'a $t<P>) -> Self {
                    Self {
                        width: HasEncoderMetadata::<(), P>::width(&metadata),
                        height: HasEncoderMetadata::<(), P>::height(&metadata),
                        sequence: HasEncoderMetadata::<(), P>::sequence(&metadata),
                        color_type: HasEncoderMetadata::<(), P>::color_type(&metadata),
                        bit_depth: HasEncoderMetadata::<(), P>::bit_depth(&metadata),
                        palette: HasEncoderMetadata::<(), P>::palette(&metadata).map(|p| p.to_vec().into_boxed_slice()),
                    }
                }
            }
        )+
    }
}

impl_from_metadata!(Image Frame ImageSequence);

impl<C: Default, P: Pixel> HasEncoderMetadata<C, P> for EncoderMetadata<P> {
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
    fn sequence(&self) -> Option<(usize, LoopCount)> {
        self.sequence
    }
    fn color_type(&self) -> ColorType {
        self.color_type
    }
    fn bit_depth(&self) -> u8 {
        self.bit_depth
    }
    fn palette(&self) -> Option<&[P::Color]> {
        self.palette.as_deref()
    }
}

impl<P: Pixel> EncoderMetadata<P> {
    /// Creates a new encoder metadata with default options given the width, height, and pixel type.
    ///
    /// # Example
    /// ```
    /// use ril::{EncoderMetadata, Rgba};
    ///
    /// # fn main() {
    /// let metadata = EncoderMetadata::<Rgba>::new(256, 256);
    /// # }
    /// ```
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            sequence: None,
            color_type: P::COLOR_TYPE,
            bit_depth: P::BIT_DEPTH,
            palette: None,
        }
    }

    /// Sets the width of the image.
    #[must_use]
    pub const fn with_width(mut self, width: u32) -> Self {
        self.width = width;
        self
    }

    /// Sets the height of the image.
    #[must_use]
    pub const fn with_height(mut self, height: u32) -> Self {
        self.height = height;
        self
    }

    /// Sets sequence information regarding the image.
    #[must_use]
    pub const fn with_sequence(mut self, frame_count: usize, loop_count: LoopCount) -> Self {
        self.sequence = Some((frame_count, loop_count));
        self
    }

    /// Anticipates the frame count of the image.
    #[must_use]
    pub fn with_frame_count(mut self, frame_count: usize) -> Self {
        self.sequence = Some((
            frame_count,
            self.sequence.map_or(LoopCount::Infinite, |(_, l)| l),
        ));
        self
    }

    /// Anticipates the loop count of the image.
    #[must_use]
    pub fn with_loop_count(mut self, loop_count: LoopCount) -> Self {
        self.sequence = Some((self.sequence.map_or(0, |(f, _)| f), loop_count));
        self
    }

    /// Sets the color type of the image.
    #[must_use]
    pub const fn with_color_type(mut self, color_type: ColorType) -> Self {
        self.color_type = color_type;
        self
    }

    /// Sets the bit depth of the pixels in the image.
    #[must_use]
    pub const fn with_bit_depth(mut self, bit_depth: u8) -> Self {
        self.bit_depth = bit_depth;
        self
    }

    /// Sets the configuration parameters for the specific encoder.
    #[must_use]
    pub const fn with_config<C: Default>(self, config: C) -> EncoderMetadataWithConfig<C, P> {
        EncoderMetadataWithConfig {
            metadata: self,
            config,
        }
    }

    /// Sets the palette of the image.
    #[must_use]
    pub fn with_palette(mut self, palette: impl Deref<Target = [P::Color]>) -> Self {
        self.palette = Some(palette.to_vec().into_boxed_slice());
        self
    }
}

/// An [`EncoderMetadata`] with additional configuration parameters for the specific encoder.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EncoderMetadataWithConfig<C: Default, P: Pixel> {
    /// The encoder metadata.
    pub metadata: EncoderMetadata<P>,
    /// The encoder configuration.
    pub config: C,
}

impl<C: Default, P: Pixel> HasEncoderMetadata<C, P> for EncoderMetadataWithConfig<C, P> {
    fn width(&self) -> u32 {
        HasEncoderMetadata::<C, _>::width(&self.metadata)
    }
    fn height(&self) -> u32 {
        HasEncoderMetadata::<C, _>::height(&self.metadata)
    }
    fn sequence(&self) -> Option<(usize, LoopCount)> {
        HasEncoderMetadata::<C, _>::sequence(&self.metadata)
    }
    fn color_type(&self) -> ColorType {
        HasEncoderMetadata::<C, _>::color_type(&self.metadata)
    }
    fn bit_depth(&self) -> u8 {
        HasEncoderMetadata::<C, _>::bit_depth(&self.metadata)
    }
    fn palette(&self) -> Option<&[P::Color]> {
        HasEncoderMetadata::<C, _>::palette(&self.metadata)
    }
    fn config(self) -> C {
        self.config
    }
}

impl<C: Default, P: Pixel> Deref for EncoderMetadataWithConfig<C, P> {
    type Target = EncoderMetadata<P>;

    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

impl<C: Default, P: Pixel> DerefMut for EncoderMetadataWithConfig<C, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.metadata
    }
}

/// Low-level encoder interface around an image format. Typically only accessed for lazy encoding.
///
/// # Encoder metadata
///
/// When preparing an encoder, you must pass in some metadata to the encoder. This metadata is
/// used to configure the encoder for encoding the image. The metadata can be:
///
/// * A static [`Image`], in which case the encoder will be prepared for encoding a static image.
/// * A [`Frame`], in which case the encoder will be prepared for encoding an image sequence.
/// * An [`ImageSequence`], in which case the encoder will be prepared for encoding an image
///   sequence.
/// * An [`EncoderMetadata`] object for more fine-grained control over the encoder.
///
/// ## Notes
///
/// - The specific metadata that is actually used and/or considered depends on the
///   encoder implementation.
/// - If you provide an image, frame, or sequence as metadata, you are only registering metadata,
///   the image or frame itself will **not** be encoded yet. You would have to make a call to
///   [`Self::encode_frame`] to encode the image or frame.
/// - Use [`EncoderMetadata::from`] to create encoder metadata from an image, frame, or sequence.
///
/// ## Examples
///
/// ```no_run
/// use ril::encodings::png::{PngEncoder, PngEncoderOptions};
/// # use ril::prelude::*;
///
/// # fn main() -> ril::Result<()> {
/// # let writer = std::io::stdout();
/// // Create a new encoder with default options given the width, height, and pixel type:
/// let encoder = PngEncoder::new(writer, EncoderMetadata::<Rgba>::new(256, 256));
///
/// // Create a new encoder will encode an animated image with 2 frames, looping infinitely:
/// let metadata = EncoderMetadata::<Rgba>::new(256, 256).with_sequence(2, LoopCount::Infinite);
/// # let writer = std::io::stdout();
/// let encoder = PngEncoder::new(writer, metadata);
///
/// // Using an image/frame/sequence as metadata:
/// # let writer = std::io::stdout();
/// let image = Image::<Rgb>::open("sample.png")?; // can be any Image, Frame, or ImageSequence
/// let encoder = PngEncoder::new(writer, &image);
///
/// // Using a frame as baseline metadata but explicitly anticipating frame count
/// // NOTE: See "Anticipating the frame and loop counts" below for more information
/// let frame = Frame::from_image(image);
/// # let writer = std::io::stdout();
/// let metadata = EncoderMetadata::from(&frame).with_sequence(2, LoopCount::Infinite);
/// let encoder = PngEncoder::new(writer, metadata);
///
/// # Ok(())
/// # }
/// ```
///
/// # Anticipating frame and loop counts
///
/// All frame and loops counts for the following encoders **must** be anticipated prior to
/// encoding:
///
/// * [`PngEncoder`]
///
/// This is a limitation caused by the way these encoders/codecs are implemented. For example, in
/// the PNG format, the frame and loop counts are stored in the header of the file, which is
/// written at the very beginning of encoding.
///
/// You can anticipate the frame and loop counts by using [`EncoderMetadata::with_sequence`].
/// If you have a reference to an image, frame, or sequence, you can use [`EncoderMetadata::from`]
/// to create encoder metadata from it:
///
/// ```no_run
/// use ril::encodings::png::{PngEncoder, PngEncoderOptions};
/// # use ril::prelude::*;
///
/// # fn main() -> ril::Result<()> {
/// # let writer = std::io::stdout();
/// // Grab our image, frame, or image sequence:
/// let frame = Frame::from_image(Image::<Rgb>::open("sample.png")?);
/// // Convert it into encoder metadata with `EncoderMetadata::from`,
/// // then anticipate the frame and loop counts with `EncoderMetadata::with_sequence`:
/// let metadata = EncoderMetadata::from(&frame).with_sequence(2, LoopCount::Infinite);
/// // Finally, create the encoder with our modified metadata:
/// let encoder = PngEncoder::new(writer, metadata);
/// # Ok(())
/// # }
/// ```
///
/// If you are not certain how many frames or loops your image will have, you can use an upper bound
/// for the frame count or loop count.
///
/// # Example
/// Lazily encode images into an animated PNG (APNG):
///
/// ```no_run
/// use std::fs::File;
/// use std::time::Duration;
/// use ril::encodings::png::PngEncoder;
/// use ril::prelude::*;
///
/// const HALF_SECOND: Duration = Duration::from_millis(500);
///
/// const RED: Rgb = Rgb::new(255, 0, 0);
/// const BLUE: Rgb = Rgb::new(0, 0, 255);
///
/// fn main() -> ril::Result<()> {
///     let mut writer = File::create("output.png")?;
///     // Prepare metadata. The PNG encoder requires we anticipate the frame count prior to encoding:
///     let metadata = EncoderMetadata::<Rgb>::new(100, 100).with_frame_count(2);
///     // Create the encoder
///     let mut encoder = PngEncoder::new(&mut writer, metadata)?;
///
///     // Frame 1: 100x100 red square for 0.5s
///     let image = Image::new(100, 100, RED);
///     encoder.add_frame(&Frame::from_image(image).with_delay(HALF_SECOND))?;
///
///     // Frame 2: 100x100 blue square for 0.5s
///     let image = Image::new(100, 100, BLUE);
///     encoder.add_frame(&Frame::from_image(image).with_delay(HALF_SECOND))?;
///
///     // Finish encoding
///     encoder.finish()
/// }
/// ```
pub trait Encoder<P: Pixel, W: Write>: Sized {
    /// The type of the configuration object used to configure options for the encoder.
    type Config: Default;

    /// Creates a new encoder and prepares it for encoding an image.
    ///
    /// **This method is only used for lazy encoding of image sequences.**
    /// - If you are encoding a static image, use [`Self::encode_static`] instead. There is no need to
    ///   call any intermediate method for encoding static images since they inherently cannot be
    ///   lazily encoded after all.
    /// - If you are encoding an [`ImageSequence`] directly, use [`Self::encode_sequence`] instead.
    ///   Since [`ImageSequence`] already contains all the frames, there is no need to lazily encode
    ///   them.
    ///
    /// # Metadata
    /// The `metadata` parameter is used to provide initial metadata to the encoder. See the
    /// [trait-level documentation][Encoder] for more information.
    ///
    /// ## Notes
    /// - The specific metadata that is actually used and/or considered depends on the
    ///   encoder implementation.
    /// - If you provide an image or frame as metadata, you are only registering metadata, the
    ///   image or frame itself will **not** be encoded yet. You would have to make a call to
    ///   [`Self::encode_frame`] to encode the image or frame.
    ///
    /// # Errors
    /// * An error occured during encoding.
    /// * The image is invalid for encoding.
    fn new(dest: W, metadata: impl HasEncoderMetadata<Self::Config, P>) -> crate::Result<Self>;

    /// Adds a frame to the encoding sequence and encodes it into the given writer.
    ///
    /// **This method is only used for lazy encoding of image sequences.**
    /// - If you are encoding a static image, use [`Self::encode_static`] instead. There is no need to
    ///   call any intermediate method for encoding static images since they inherently cannot be
    ///   lazily encoded after all.
    /// - If you are encoding an [`ImageSequence`] directly, use [`Self::encode_sequence`] instead.
    ///   Since [`ImageSequence`] already contains all the frames, there is no need to lazily encode
    ///   them.
    ///
    /// # Errors
    /// * An error occured during encoding.
    fn add_frame(&mut self, frame: &impl FrameLike<P>) -> crate::Result<()>;

    /// Finishes encoding the image. This **must** be called once after all frames are encoded.
    ///
    /// # Errors
    /// * An error occured during encoding or writing.
    fn finish(self) -> crate::Result<()>;

    /// Encodes a static image into the given writer.
    ///
    /// # Errors
    /// * An error occured during encoding.
    fn encode_static(image: &Image<P>, dest: W) -> crate::Result<()> {
        let mut encoder = Self::new(dest, image)?;
        encoder.add_frame(image)?;
        encoder.finish()
    }

    /// Encodes the given image sequence into the given writer.
    ///
    /// # Errors
    /// * An error occured during encoding.
    fn encode_sequence(sequence: &ImageSequence<P>, dest: W) -> crate::Result<()> {
        let mut encoder = Self::new(dest, sequence)?;
        for frame in sequence.iter() {
            encoder.add_frame(frame)?;
        }
        encoder.finish()
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
    fn loop_count(&self) -> LoopCount;

    /// Collects all frames in this iterator and turns it into a high level [`ImageSequence`].
    /// If any frame fails, that error is returned.
    ///
    /// # Errors
    /// * An error occured during decoding one of the frames.
    fn into_sequence(self: Box<Self>) -> crate::Result<ImageSequence<P>> {
        let loop_count = self.loop_count();
        let frames = self.collect::<crate::Result<Vec<_>>>()?;

        Ok(ImageSequence::from_frames(frames).with_loop_count(loop_count))
    }
}

/// Represents a single static image wrapped in a frame iterator.
#[allow(clippy::large_enum_variant)]
pub struct SingleFrameIterator<P: Pixel>(Option<Image<P>>);

impl<P: Pixel> SingleFrameIterator<P> {
    /// Create a new single static image frame iterator.
    #[must_use]
    pub const fn new(image: Image<P>) -> Self {
        Self(Some(image))
    }
}

impl<P: Pixel> FrameIterator<P> for SingleFrameIterator<P> {
    fn len(&self) -> u32 {
        1
    }
    fn loop_count(&self) -> LoopCount {
        LoopCount::Exactly(1)
    }

    fn into_sequence(mut self: Box<Self>) -> crate::Result<ImageSequence<P>> {
        let image = self.0.take().unwrap();
        let frame = Frame::from_image(image);

        Ok(ImageSequence::new().with_frame(frame))
    }
}

impl<P: Pixel> Iterator for SingleFrameIterator<P> {
    type Item = crate::Result<Frame<P>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.take().map(|image| Ok(Frame::from_image(image)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.0.is_some() {
            (1, Some(1))
        } else {
            (0, Some(0))
        }
    }
}
