use crate::{Image, Pixel};
use std::time::Duration;

/// The method used to dispose a frame before transitioning to the next frame in an image sequence.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum DisposalMethod {
    /// Do not dispose the current frame. Usually not desired for transparent images.
    #[default]
    None,
    /// Dispose the current frame completely and replace it with the image's background color.
    Background,
    /// Dispose and replace the current frame with the previous frame.
    Previous,
}

/// Represents a frame in an image sequence. It encloses an [`Image`] and extra metadata
/// about the frame.
#[derive(Clone)]
pub struct Frame<P: Pixel> {
    inner: Image<P>,
    delay: Duration,
    disposal: DisposalMethod,
}

impl<P: Pixel> Frame<P> {
    /// Creates a new frame with the given image and default metadata.
    pub fn from_image(image: Image<P>) -> Self {
        Self {
            inner: image,
            delay: Duration::default(),
            disposal: DisposalMethod::default(),
        }
    }

    /// Sets the frame delay to the given duration.
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Sets the disposal method for this frame when transitioning to the next.
    pub fn with_disposal(mut self, disposal: DisposalMethod) -> Self {
        self.disposal = disposal;
        self
    }

    /// Returns a reference to the image this frame contains.
    pub fn image(&self) -> &Image<P> {
        &self.inner
    }

    /// Consumes this frame returning the inner image it represents.
    pub fn into_image(self) -> Image<P> {
        self.inner
    }

    /// Returns the width of this frame.
    pub fn width(&self) -> u32 {
        self.inner.width()
    }

    /// Returns the height of this frame.
    pub fn height(&self) -> u32 {
        self.inner.height()
    }

    /// Returns the dimensions of this frame.
    pub fn dimensions(&self) -> (u32, u32) {
        self.inner.dimensions()
    }

    /// Returns the delay duration for this frame.
    pub fn delay(&self) -> Duration {
        self.delay
    }

    /// Returns the disposal method for this frame.
    pub fn disposal(&self) -> DisposalMethod {
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

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum LoopCount {
    /// Loops infinitely.
    #[default]
    Infinite,
    /// Loops the specified amount of times.
    Exactly(u32),
}

impl LoopCount {
    /// Returns the exact number of times this loop should be repeated or 0.
    pub fn count_or_zero(self) -> u32 {
        match self {
            LoopCount::Infinite => 0,
            LoopCount::Exactly(count) => count,
        }
    }
}

/// Represents a sequence of image frames such as an animated image.
///
/// See [`Image`] for the static image counterpart, and see [`Frame`] to see how each frame
/// is represented in an image sequence.
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

impl<P: Pixel> ImageSequence<P> {
    /// Creates a new image sequence with no frames.
    ///
    /// # Note
    /// A frameless image sequence is forbidden to be encoded and you will receive a panic.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new image sequence from the given frames.
    pub fn from_frames(frames: Vec<Frame<P>>) -> Self {
        Self { frames, ..Self::default() }
    }

    /// Adds a new frame to this image sequence and returns this sequence. Useful for
    /// method-chaining.
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
    pub fn loop_count(&self) -> LoopCount {
        self.loops
    }

    /// Sets how many times this image sequence loops for.
    pub fn with_loop_count(mut self, loops: LoopCount) -> Self {
        self.loops = loops;
        self
    }

    /// Sets the exact number of loops this image sequence loops for.
    pub fn looped_exactly(mut self, loops: u32) -> Self {
        self.with_loop_count(LoopCount::Exactly(loops))
    }

    /// Sets the image sequence to loop infinitely.
    pub fn looped_infinitely(mut self) -> Self {
        self.with_loop_count(LoopCount::Infinite)
    }

    /// Consumes this image sequence and returns the frames it contains.
    pub fn into_frames(self) -> Vec<Frame<P>> {
        self.frames
    }

    /// Iterates through the frames in this image sequence by reference.
    pub fn iter(&self) -> impl Iterator<Item = &Frame<P>> {
        self.frames.iter()
    }

    /// Returns the number of frames in this image sequence.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Consumes this image sequence and returns the first image.
    pub fn into_first_image(self) -> Image<P> {
        self.frames[0].into_image()
    }

    /// Returns a reference to the first frame in the image sequence.
    pub fn first_frame(&self) -> &Frame<P> {
        &self.frames[0]
    }
}
