//! Implements the animated image and image sequence interface.

use crate::{Error, FrameIterator, Image, ImageFormat, Pixel, Result};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    time::Duration,
};

/// The method used to dispose a frame before transitioning to the next frame in an image sequence.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DisposalMethod {
    /// Do not dispose the current frame. Usually not desired for transparent images.
    None,
    /// Dispose the current frame completely and replace it with the image's background color.
    Background,
    /// Dispose and replace the current frame with the previous frame.
    Previous,
}

impl Default for DisposalMethod {
    fn default() -> Self {
        Self::None
    }
}

/// Represents a frame in an image sequence. It encloses an [`Image`] and extra metadata
/// about the frame.
///
/// # Support for paletted images
/// Frames representing paletted images are currently unsupported. See documentation of
/// [`ImageSequence`] for more information.
///
/// # See Also
/// * [`ImageSequence`] for more information about image sequences.
#[derive(Clone)]
pub struct Frame<P: Pixel> {
    inner: Image<P>,
    delay: Duration,
    disposal: DisposalMethod,
}

impl<P: Pixel> Frame<P> {
    /// Creates a new frame with the given image and default metadata.
    #[must_use]
    pub fn from_image(image: Image<P>) -> Self {
        Self {
            inner: image,
            delay: Duration::default(),
            disposal: DisposalMethod::default(),
        }
    }

    /// Sets the frame delay to the given duration in place.
    pub fn set_delay(&mut self, delay: Duration) {
        self.delay = delay;
    }

    /// Takes this frame and sets the frame delay to the given duration.
    #[must_use]
    pub const fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Sets the disposal method for this frame in place.
    pub fn set_disposal(&mut self, disposal: DisposalMethod) {
        self.disposal = disposal;
    }

    /// Takes this frame and sets the disposal method for this frame when transitioning to the next.
    #[must_use]
    pub const fn with_disposal(mut self, disposal: DisposalMethod) -> Self {
        self.disposal = disposal;
        self
    }

    /// Returns a reference to the image this frame contains.
    #[must_use]
    pub const fn image(&self) -> &Image<P> {
        &self.inner
    }

    /// Maps the inner image to the given function.
    #[must_use]
    pub fn map_image<T: Pixel>(self, f: impl FnOnce(Image<P>) -> Image<T>) -> Frame<T> {
        Frame {
            inner: f(self.inner),
            delay: self.delay,
            disposal: self.disposal,
        }
    }

    /// Returns a mutable reference to the image this frame contains.
    pub fn image_mut(&mut self) -> &mut Image<P> {
        &mut self.inner
    }

    /// Consumes this frame returning the inner image it represents.
    #[allow(clippy::missing_const_for_fn)] // can't use destructors with const fn
    #[must_use]
    pub fn into_image(self) -> Image<P> {
        self.inner
    }

    /// Returns the delay duration for this frame.
    #[must_use]
    pub const fn delay(&self) -> Duration {
        self.delay
    }

    /// Returns the disposal method for this frame.
    #[must_use]
    pub const fn disposal(&self) -> DisposalMethod {
        self.disposal
    }
}

impl<P: Pixel> From<Image<P>> for Frame<P> {
    fn from(image: Image<P>) -> Self {
        Self::from_image(image)
    }
}

impl<P: Pixel> From<Frame<P>> for Image<P> {
    fn from(frame: Frame<P>) -> Self {
        frame.into_image()
    }
}

impl<P: Pixel> std::ops::Deref for Frame<P> {
    type Target = Image<P>;

    fn deref(&self) -> &Self::Target {
        self.image()
    }
}

impl<P: Pixel> std::ops::DerefMut for Frame<P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.image_mut()
    }
}

/// Determines how many times an image sequence should repeat itself, or if it
/// should repeat infinitely.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LoopCount {
    /// Loops infinitely.
    Infinite,
    /// Loops the specified amount of times.
    Exactly(u32),
}

impl Default for LoopCount {
    fn default() -> Self {
        Self::Infinite
    }
}

impl LoopCount {
    /// Returns the exact number of times this loop should be repeated or 0.
    #[must_use]
    pub const fn count_or_zero(self) -> u32 {
        match self {
            Self::Infinite => 0,
            Self::Exactly(count) => count,
        }
    }
}

/// Represents a sequence of image frames such as an animated image.
///
/// # See Also
/// * [`Image`] for the static image counterpart
/// * [`Frame`] to see how each frame is represented in an image sequence.
#[derive(Clone, Default)]
pub struct ImageSequence<P: Pixel> {
    frames: Vec<Frame<P>>,
    loops: LoopCount,
}

impl<P: Pixel> IntoIterator for ImageSequence<P> {
    type Item = Frame<P>;
    type IntoIter = std::vec::IntoIter<Frame<P>>;

    fn into_iter(self) -> Self::IntoIter {
        self.frames.into_iter()
    }
}

impl<P: Pixel> FromIterator<Frame<P>> for ImageSequence<P> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Frame<P>>,
    {
        Self::from_frames(iter.into_iter().collect())
    }
}

impl<P: Pixel> FromIterator<Result<Frame<P>>> for ImageSequence<P> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Result<Frame<P>>>,
    {
        Self::from_frames(iter.into_iter().collect::<Result<Vec<_>>>().unwrap())
    }
}

impl<P: Pixel> ImageSequence<P> {
    /// Creates a new image sequence with no frames.
    ///
    /// # Note
    /// A frameless image sequence is forbidden to be encoded and you will receive a panic.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Decodes the image sequence with the explicitly given image encoding from the raw byte
    /// reader.
    ///
    /// This decodes frames lazily as an iterator. Call [`DynamicFrameIterator::into_sequence`] to
    /// collect all frames greedily into an [`ImageSequence`].
    ///
    /// If the image sequence is a single-frame static image or if the encoding format does not
    /// support animated images, this will just return an image sequence containing one frame.
    ///
    /// # Errors
    /// * `DecodingError`: The image could not be decoded, maybe it is corrupt.
    pub fn from_read<'a, R: Read + 'a>(
        format: ImageFormat,
        bytes: R,
    ) -> Result<Box<dyn FrameIterator<P> + 'a>>
    where
        P: 'a,
    {
        format.run_sequence_decoder(bytes)
    }

    /// Decodes an image sequence from the given read stream of bytes, inferring its encoding.
    ///
    /// This decodes frames lazily as an iterator. Call [`DynamicFrameIterator::into_sequence`] to
    /// collect all frames greedily into an [`ImageSequence`].
    ///
    /// If the image sequence is a single-frame static image or if the encoding format does not
    /// support animated images, this will just return an image sequence containing one frame.
    ///
    /// # Note
    /// The bound on `bytes` includes `Write` due to a Rust limitation. This will be looked into
    /// in the future to not require `Write`.
    ///
    /// If you are limited by this trait bound, you can either specify the image format manually
    /// using [`from_read`], or you can try using [`ImageFormat::infer_encoding`] along with
    /// [`from_read`] manually instead. If you are able to use [`from_bytes`] instead, which takes
    /// a byte slice instead of a `Read` stream, you can either that or [`from_bytes_inferred`],
    /// too, which does not require a `Write` bound either.
    ///
    /// # Errors
    /// * `DecodingError`: The image could not be decoded, maybe it is corrupt.
    /// * `UnknownEncodingFormat`: Could not infer the encoding from the image. Try explicitly
    ///   specifying it.
    ///
    /// # Panics
    /// * No decoder implementation for the given encoding format.
    pub fn from_read_inferred<'a, R: Read + Write + 'a>(
        mut bytes: R,
    ) -> Result<Box<dyn FrameIterator<P> + 'a>>
    where
        P: 'a,
    {
        let mut buffer = Vec::new();
        bytes.read_to_end(&mut buffer)?;

        match ImageFormat::infer_encoding(&buffer) {
            ImageFormat::Unknown => Err(Error::UnknownEncodingFormat),
            format => {
                bytes.write_all(&buffer)?;
                format.run_sequence_decoder(bytes)
            }
        }
    }

    /// Decodes an image sequence with the explicitly given image encoding from the byte slice.
    /// Could be useful in conjunction with the `include_bytes!` macro.
    ///
    /// Currently, this is not any different than [`from_read`].
    ///
    /// This decodes frames lazily as an iterator. Call [`DynamicFrameIterator::into_sequence`] to
    /// collect all frames greedily into an [`ImageSequence`].
    ///
    /// If the image sequence is a single-frame static image or if the encoding format does not
    /// support animated images, this will just return an image sequence containing one frame.
    ///
    /// # Note
    /// This takes different parameters than [`Image::from_bytes`] - that takes any `AsRef<[u8]>`
    /// while this strictly only takes byte slices (`&[u8]`).
    ///
    /// # Errors
    /// * `DecodingError`: The image could not be decoded, maybe it is corrupt.
    ///
    /// # Panics
    /// * No decoder implementation for the given encoding format.
    pub fn from_bytes<'a>(
        format: ImageFormat,
        bytes: &'a [u8],
    ) -> Result<Box<dyn FrameIterator<P> + 'a>>
    where
        P: 'a,
    {
        format.run_sequence_decoder(bytes)
    }

    /// Decodes an image sequence from the given byte slice, inferring its encoding.
    /// Could be useful in conjunction with the `include_bytes!` macro.
    ///
    /// This is more efficient than [`from_read_inferred`], and can act as a workaround for
    /// bypassing the `Write` trait bound.
    ///
    /// This decodes frames lazily as an iterator. Call [`DynamicFrameIterator::into_sequence`] to
    /// collect all frames greedily into an [`ImageSequence`].
    ///
    /// If the image sequence is a single-frame static image or if the encoding format does not
    /// support animated images, this will just return an image sequence containing one frame.
    ///
    /// # Note
    /// This takes different parameters than [`Image::from_bytes`] - that takes any `AsRef<[u8]>`
    /// while this strictly only takes byte slices (`&[u8]`).
    ///
    /// # Errors
    /// * `DecodingError`: The image could not be decoded, maybe it is corrupt.
    /// * `UnknownEncodingFormat`: Could not infer the encoding from the image. Try explicitly
    ///   specifying it.
    ///
    /// # Panics
    /// * No decoder implementation for the given encoding format.
    pub fn from_bytes_inferred<'a>(bytes: &'a [u8]) -> Result<Box<dyn FrameIterator<P> + 'a>>
    where
        P: 'a,
    {
        match ImageFormat::infer_encoding(bytes) {
            ImageFormat::Unknown => Err(Error::UnknownEncodingFormat),
            format => format.run_sequence_decoder(bytes),
        }
    }

    /// Opens a file from the given path and decodes it, returning an iterator over its frames.
    ///
    /// The encoding of the image is automatically inferred. You can explicitly pass in an encoding
    /// by using the [`from_reader`] method.
    ///
    /// # Note
    /// Unlike the inference of [`Image::open`] this does **not** infer from raw bytes if inferring
    /// from file extension fails; instead it immediately returns the error.
    ///
    /// # Errors
    /// todo!()
    pub fn open<'a>(path: impl AsRef<Path> + 'a) -> Result<Box<dyn FrameIterator<P> + 'a>>
    where
        P: 'a,
    {
        let file = File::open(path.as_ref())?;

        let format = match ImageFormat::from_path(path)? {
            ImageFormat::Unknown => return Err(Error::UnknownEncodingFormat),
            format => format,
        };

        format.run_sequence_decoder(file)
    }

    /// Encodes this image sequence with the given encoding and writes it to the given write buffer.
    ///
    /// # Errors
    /// * An error occured during encoding.
    ///
    /// # Panics
    /// * No encoder implementation for the given encoding format.
    pub fn encode(&self, encoding: ImageFormat, dest: &mut impl Write) -> Result<()> {
        encoding.run_sequence_encoder(self, dest)
    }

    /// Saves the image sequence with the given encoding to the given path.
    /// You can try saving to a memory buffer by using the [`encode`] method.
    ///
    /// # Errors
    /// * An error occured during encoding.
    ///
    /// # Panics
    /// * No encoder implementation for the given encoding format.
    pub fn save(&self, encoding: ImageFormat, path: impl AsRef<Path>) -> Result<()> {
        let mut file = File::create(path).map_err(Error::IoError)?;
        self.encode(encoding, &mut file)
    }

    /// Saves the image sequence to the given path, inferring the encoding from the given
    /// path/filename extension.
    ///
    /// This is obviously slower than [`save`] since this method itself uses it. You should only
    /// use this method if the filename is dynamic, or if you do not know the desired encoding
    /// before runtime.
    ///
    /// See [`save`] for more information on how saving works.
    ///
    /// # Errors
    /// * Could not infer encoding format.
    /// * An error occured during encoding.
    ///
    /// # Panics
    /// * No encoder implementation for the given encoding format.
    pub fn save_inferred(&self, path: impl AsRef<Path>) -> Result<()> {
        let encoding = ImageFormat::from_path(path.as_ref())?;

        match encoding {
            ImageFormat::Unknown => Err(Error::UnknownEncodingFormat),
            _ => self.save(encoding, path),
        }
    }

    /// Creates a new image sequence from the given frames.
    #[must_use]
    pub fn from_frames(frames: Vec<Frame<P>>) -> Self {
        Self {
            frames,
            ..Self::default()
        }
    }

    /// Adds a new frame to this image sequence and returns this sequence. Useful for
    /// method-chaining.
    #[must_use]
    pub fn with_frame(mut self, frame: Frame<P>) -> Self {
        self.frames.push(frame);
        self
    }

    /// Adds a new frame to this image sequence.
    pub fn push_frame(&mut self, frame: Frame<P>) {
        self.frames.push(frame);
    }

    /// Extends frames from the given iterator.
    pub fn extend_frames<I>(&mut self, frames: I)
    where
        I: IntoIterator<Item = Frame<P>>,
    {
        self.frames.extend(frames);
    }

    /// Returns how many times this image sequence loops for.
    #[must_use]
    pub const fn loop_count(&self) -> LoopCount {
        self.loops
    }

    /// Takes this image and sets how many times this image sequence loops for.
    #[must_use]
    pub const fn with_loop_count(mut self, loops: LoopCount) -> Self {
        self.loops = loops;
        self
    }

    /// Sets how many times this image sequence loops for in place.
    pub fn set_loop_count(&mut self, loops: LoopCount) {
        self.loops = loops;
    }

    /// Sets the exact number of loops this image sequence loops for.
    #[must_use]
    pub const fn looped_exactly(self, loops: u32) -> Self {
        self.with_loop_count(LoopCount::Exactly(loops))
    }

    /// Sets the image sequence to loop infinitely.
    #[must_use]
    pub const fn looped_infinitely(self) -> Self {
        self.with_loop_count(LoopCount::Infinite)
    }

    /// Consumes this image sequence and returns the frames it contains.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn into_frames(self) -> Vec<Frame<P>> {
        self.frames
    }

    /// Iterates through the frames in this image sequence by reference.
    pub fn iter(&self) -> impl Iterator<Item = &Frame<P>> {
        self.frames.iter()
    }

    /// Iterates through the frames in this image sequence by mutable reference.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Frame<P>> {
        self.frames.iter_mut()
    }

    /// Returns whether there are no frames in the image sequence. If so, this will probably be
    /// invalid to encode.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Returns the number of frames in this image sequence.
    #[must_use]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Consumes this image sequence and returns the first image.
    ///
    /// # Panics
    /// * The image sequence is empty.
    #[must_use]
    pub fn into_first_image(self) -> Image<P> {
        self.into_frames().swap_remove(0).into_image()
    }

    /// Returns a reference to the first frame in the image sequence, if any.
    #[must_use]
    pub fn first_frame(&self) -> Option<&Frame<P>> {
        self.frames.first()
    }

    /// Returns a reference to the first frame in the image sequence. This does not check if there
    /// are no frames in the image sequence.
    ///
    /// # Safety
    /// You must guarantee that there is at least one frame in the image sequence.
    #[must_use]
    pub unsafe fn first_frame_unchecked(&self) -> &Frame<P> {
        self.frames.get_unchecked(0)
    }

    /// Returns a mutable reference to the first frame in the image sequence, if any.
    #[must_use]
    pub fn first_frame_mut(&mut self) -> Option<&mut Frame<P>> {
        self.frames.get_mut(0)
    }

    /// Returns a mutable reference to the first frame in the image sequence. This does not check if
    /// there are no frames in the image sequence.
    ///
    /// # Safety
    /// You must guarantee that there is at least one frame in the image sequence.
    #[must_use]
    pub unsafe fn first_frame_unchecked_mut(&mut self) -> &mut Frame<P> {
        self.frames.get_unchecked_mut(0)
    }
}
