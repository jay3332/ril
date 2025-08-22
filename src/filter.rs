//! Filters that can be applied on images.

use crate::{BitPixel, FromVector, Image, IntoVector, Pixel, Vector};
use num_traits::{
    AsPrimitive, Bounded, ConstOne, ConstZero, Float, FromPrimitive, Num, SaturatingAdd, Zero,
};
use std::{
    marker::PhantomData,
    ops::{Add, Div, Mul},
};

/// An image filter than can be lazily applied to an image or a filtered image.
///
/// Filters make the following guarantees:
/// * They preserve the image dimensions.
/// * They modify single pixels at a time. They can use surrounding pixels for context, but they
///   cannot modify them.
pub trait Filter {
    /// The pixel type of the input image.
    type Input: Pixel;
    /// The pixel type of the output image.
    type Output: Pixel;

    /// Applies the filter to the given pixel.
    fn apply_pixel(
        &self,
        image: &Image<Self::Input>,
        x: u32,
        y: u32,
        pixel: Self::Input,
    ) -> Self::Output;
}

/// An identity filter. This filter does not modify the image in any way.
#[derive(Copy, Clone, Debug, Default)]
pub struct Identity<P: Pixel> {
    _marker: PhantomData<P>,
}

impl<P: Pixel> Identity<P> {
    /// Creates a new identity filter.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<P: Pixel> Filter for Identity<P> {
    type Input = P;
    type Output = P;

    fn apply_pixel(
        &self,
        _image: &Image<Self::Input>,
        _x: u32,
        _y: u32,
        pixel: Self::Input,
    ) -> Self::Output {
        pixel
    }
}

/// A filter which applies the given filter only to the given mask.
///
/// This filter is useful for applying effects to specific parts of an image, such as blurring or sharpening
/// while leaving other parts unchanged.
///
/// This filter will panic if the mask is not the same size as the image being filtered.
pub struct Mask<'mask, F: Filter> {
    /// The filter to apply.
    pub filter: F,
    /// The mask to apply the filter to.
    pub mask: &'mask Image<BitPixel>,
}

impl<'mask, F: Filter> Mask<'mask, F> {
    /// Creates a new mask filter.
    #[must_use]
    pub fn new(filter: F, mask: &'mask Image<BitPixel>) -> Self {
        Self { filter, mask }
    }
}

impl<'mask, F: Filter> Filter for Mask<'mask, F>
where
    F::Output: From<F::Input>,
{
    type Input = F::Input;
    type Output = F::Output;

    fn apply_pixel(
        &self,
        image: &Image<Self::Input>,
        x: u32,
        y: u32,
        pixel: Self::Input,
    ) -> Self::Output {
        if self.mask.get_pixel(x, y).is_some_and(BitPixel::value) {
            self.filter.apply_pixel(image, x, y, pixel)
        } else {
            pixel.into()
        }
    }
}

/// Represents a filter which is backed by a kernel matrix, a 2D array of weights. A matrix
/// convolution is performed between the kernel and the image to produce an output image.
///
/// # Matrix Convolution
///
/// Let $\vec{I}(x, y)$ be the pixel at coordinates $(x, y)$ of the input image.
/// Let $\vec{O}(x, y)$ be the pixel at coordinates $(x, y)$ of the output image.
/// Finally, let the kernel matrix $\mathbf{K}$ of have weights $\mathbf{K}_{i,j}$, where
/// $-a <= i <= a$ and $-b <= j <= b$.
///
/// Then, the output pixel is computed as: $
///     \vec{O}(x, y) = \mathbf{K} \ast \vec{I}(x, y) =
///     \sum_{i=-a}^{a} \sum_{j=-b}^{b} k_{i,j} \vec{I}(x-i, y-j).
/// $
/// where: $\ast$ is the convolution operator.
trait Kernel<E: Copy + 'static>: Filter {
    /// Creates a new kernel with all weights set to zero.
    fn zero() -> Self
    where
        Self: Sized,
        E: ConstZero + Copy;

    /// Returns the dimensions of the kernel.
    fn dimensions(&self) -> (usize, usize);

    /// Returns the center indices of the kernel, corresponding to the pixel being processed.
    fn center(&self) -> (usize, usize);

    /// Returns the weight at the given position in the kernel matrix, $\mathbf{K}_{i,j}$.
    fn weight(&self, i: usize, j: usize) -> E;

    /// Sets the weight at the given position in the kernel matrix, $\mathbf{K}_{i,j}$.
    fn set_weight(&mut self, i: usize, j: usize, weight: E);

    /// Applies the given operation to the kernel in place.
    fn apply(&mut self, f: impl Fn(usize, usize, E) -> E) {
        let (height, width) = self.dimensions();
        for i in 0..height {
            for j in 0..width {
                let weight = self.weight(i, j);
                let new_weight = f(i, j, weight);
                self.set_weight(i, j, new_weight);
            }
        }
    }

    /// Returns a flattened iterator over the kernel weights.
    fn iter_weights(&self) -> impl Iterator<Item = E> + '_ {
        let (height, width) = self.dimensions();
        (0..height).flat_map(move |i| (0..width).map(move |j| self.weight(i, j)))
    }

    /// Computes the sum of all weights in the kernel. If this sum is not ``1``, then the output
    /// image will have a different "mean brightness" than the input image.
    fn sum(&self) -> E
    where
        E: Zero + Add<Output = E> + Copy,
    {
        self.iter_weights().fold(E::zero(), |acc, x| acc + x)
    }

    /// Adds this kernel to another kernel of the same type, producing a new kernel.
    fn add(mut self, other: Self) -> Self
    where
        Self: Sized,
        E: Add<Output = E> + Copy,
    {
        debug_assert_eq!(
            self.dimensions(),
            other.dimensions(),
            "Kernels must have the same dimensions",
        );
        self.apply(|i, j, weight| weight + other.weight(i, j));
        self
    }

    /// Multiplies this kernel by a scalar, producing a new kernel.
    fn scale(mut self, scalar: E) -> Self
    where
        Self: Sized,
        E: Mul<Output = E> + Copy,
    {
        self.apply(|_, _, weight| weight * scalar);
        self
    }

    /// Returns a normalized version of the kernel such that the sum of all weights is 1.
    /// This is useful for ensuring that the brightness of the output image is maintained.
    fn normalize(mut self) -> Self
    where
        Self: Sized,
        E: Zero + Add<Output = E> + Div<Output = E> + Copy + PartialOrd,
    {
        let sum = self.sum();
        if sum != E::zero() {
            self.apply(|_, _, weight| weight / sum);
        }
        self
    }
}

/// A statically-sized filter that performs a convolution between the given fixed-size kernel matrix
/// and the image.
///
/// This is a gateway to effects such as blurring, sharpening, and edge detection, where
/// each output pixel is a function of the input pixel and its neighbors.
///
/// The kernel matrix is a 2D array of weights that defines how much each neighboring pixel
/// contributes to the output pixel. The center of the kernel matrix corresponds to the pixel being
/// processed, and the surrounding pixels are its neighbors.
///
/// # Type Parameters
///
/// - `KERNEL_WIDTH`: The width of the kernel matrix (number of columns).
/// - `KERNEL_HEIGHT`: The height of the kernel matrix (number of rows).
/// - `N`: The number of channels in the input and output pixels (e.g., 3 for RGB).
/// - `Input`: The pixel type of the input image, which must implement `Pixel` and `IntoVector<N>`.
/// - `Output`: The pixel type of the output image, which must implement `Pixel` and `FromVector<N>`.
/// - `Element`: The type of each kernel weight (typically a floating-point type like `f32` or `f64`).
///
/// # Note
///
/// The size of the kernel must be known at compile-time. If you need a convolution filter with
/// dynamically sized kernels, consider using [`DynamicConvolution`].
///
/// # Example
///
/// This performs a box blur filter on an image, which averages the pixel values in a rectangular
/// region around each pixel:
///
/// ```no_run
/// # use ril::prelude::*;
/// # fn main() -> ril::Result<()> {
/// // Create a convolution filter with a 7x7 kernel for box blur
/// const BLUR: Convolution<7, 7, 3, Rgb> = Convolution::box_blur();
///
/// // Load an image, apply the filter, and save the result
/// Image::<Rgb>::open("puffins.jpg")?
///    .filtered(&BLUR)
///    .save_inferred("blurry_puffins.png")?;
/// # Ok(())
/// # }
/// ```
///
/// # See Also
///
/// - [`Kernel`] for the trait that defines the kernel interface.
/// - [`DynamicConvolution`] for a convolution filter with a dynamically allocated kernel.
#[derive(Clone, Debug)]
pub struct Convolution<
    const KERNEL_WIDTH: usize,
    const KERNEL_HEIGHT: usize,
    const N: usize,
    Input: Pixel + IntoVector<N>,
    Output: Pixel + FromVector<N> = Input,
    Element: Copy + AsPrimitive<Output::Element> + 'static = f32,
> where
    Input::Element: AsPrimitive<Element>,
    Output::Element: Copy + 'static,
{
    /// The kernel, which is the matrix of weights to apply to the output pixel.
    pub kernel: [[Element; KERNEL_WIDTH]; KERNEL_HEIGHT],
    _marker: PhantomData<(Input, Output)>,
}

impl<const KERNEL_WIDTH: usize, const KERNEL_HEIGHT: usize, const N: usize, I, O, E>
    Convolution<KERNEL_WIDTH, KERNEL_HEIGHT, N, I, O, E>
where
    I: Pixel + IntoVector<N>,
    O: Pixel + FromVector<N>,
    E: Copy + AsPrimitive<O::Element>,
    I::Element: AsPrimitive<E>,
    O::Element: Copy,
{
    /// Creates a new convolution filter with the given kernel.
    ///
    /// # Example
    /// ```no_run
    /// # use ril::Image;
    /// # use ril::filter::Convolution;
    /// # use ril::pixel::Rgb;
    /// // Create a 3x3 kernel for a simple edge detection filter
    /// const KERNEL: Convolution<3, 3, 3, Rgb> = Convolution::new([
    ///     [0.0, -1.0, 0.0],
    ///     [-1.0, 4.0, -1.0],
    ///     [0.0, -1.0, 0.0],
    /// ]);
    ///
    /// // Use the kernel in an image processing operation
    /// # fn main() -> ril::Result<()> {
    /// Image::<Rgb>::open("sample.png")?
    ///     .filtered(&KERNEL)
    ///     .save_inferred("output.png")?;
    /// # Ok(())
    /// # }
    #[must_use]
    pub const fn new(kernel: [[E; KERNEL_WIDTH]; KERNEL_HEIGHT]) -> Self {
        Self {
            kernel,
            _marker: PhantomData,
        }
    }

    /// Returns the center indices of the kernel, corresponding to the pixel being processed.
    #[must_use]
    pub const fn center() -> (usize, usize) {
        (KERNEL_HEIGHT / 2, KERNEL_WIDTH / 2)
    }

    /// Creates a zero-weighted convolution filter, which "wipes" the image, setting all output
    /// pixels to zero.
    #[must_use]
    pub const fn zero() -> Self
    where
        E: ConstZero + Copy,
    {
        Self::new([[E::ZERO; KERNEL_WIDTH]; KERNEL_HEIGHT])
    }

    /// Returns the kernel weight vector at the given position in the kernel matrix,
    /// $\mathbf{K}_{i,j}$.
    #[inline]
    pub const fn weight(&self, i: usize, j: usize) -> E {
        debug_assert!(
            i < KERNEL_HEIGHT && j < KERNEL_WIDTH,
            "Indices out of bounds"
        );
        self.kernel[i][j]
    }

    /// Normalizes the kernel by dividing each element by the sum of all elements in the kernel.
    pub fn normalize(&mut self)
    where
        E: ConstZero + SaturatingAdd + PartialOrd + Div<Output = E>,
    {
        let sum = self
            .kernel
            .iter()
            .flat_map(|row| row.iter())
            .fold(E::ZERO, |acc, n| acc.saturating_add(n));

        for entry in self.kernel.iter_mut().flat_map(|row| row.iter_mut()) {
            *entry = if sum != E::ZERO {
                *entry / sum
            } else {
                E::ZERO
            };
        }
    }

    /// Returns a normalized version of the filter such that the sum of all weights is 1.
    pub fn normalized(mut self) -> Self
    where
        E: ConstZero + SaturatingAdd + PartialOrd + Div<Output = E>,
    {
        self.normalize();
        self
    }

    /// Creates an identity convolution filter, which does not modify the image.
    ///
    /// # Example
    /// ```
    /// # use ril::{Image, Rgb, filter::Convolution};
    /// # fn main() -> ril::Result<()> {
    /// let image = Image::<Rgb>::from_bytes_inferred(include_bytes!("../tests/sample.png"))?;
    ///
    /// let filter = Convolution::identity();
    /// image.filtered(&filter).save_inferred("../tests/out/docs_convolution_identity.png")?;
    /// # Ok(())
    /// # }
    #[must_use]
    pub const fn identity() -> Self
    where
        E: ConstZero + ConstOne + Copy,
    {
        let (i, j) = Self::center();
        let mut kernel = [[E::ZERO; KERNEL_WIDTH]; KERNEL_HEIGHT];
        kernel[i][j] = E::ONE;
        Self::new(kernel)
    }

    /// Creates a convolution filter that applies a box blur effect to the image.
    ///
    /// This filter averages the pixel values in a square region around each pixel to calculate
    /// output pixels.
    #[must_use]
    pub fn box_blur() -> Self
    where
        E: ConstOne + Float + FromPrimitive,
    {
        let size = E::from_usize(KERNEL_WIDTH * KERNEL_HEIGHT).unwrap();
        let element = E::ONE / size;
        Self::new([[element; KERNEL_WIDTH]; KERNEL_HEIGHT])
    }
}

macro_rules! impl_convolution_apply_pixel {
    (
        $self:ident, $image:ident, $x:ident, $y:ident;
        $center:expr, $w:expr, $h:expr
    ) => {{
        let image = $image;
        let (x, y) = ($x, $y);

        let (center_y, center_x) = $center;
        let mut output = Vector::zero();

        for i in 0..$h {
            for j in 0..$w {
                let kernel_value = $self.weight(i, j);
                let neighbor_x = x.saturating_sub(center_x as u32).saturating_add(j as u32);
                let neighbor_y = y.saturating_sub(center_y as u32).saturating_add(i as u32);

                if let Some(neighbor_pixel) = image.get_pixel(neighbor_x, neighbor_y) {
                    output += neighbor_pixel.into_vector().map(AsPrimitive::as_) * kernel_value;
                }
            }
        }

        Self::Output::from_vector(output.map(|e| {
            let min = <O::Element as Bounded>::min_value().as_();
            let max = <O::Element as Bounded>::max_value().as_();
            num_traits::clamp(e, min, max).as_()
        }))
    }};
}

impl<const KERNEL_WIDTH: usize, const KERNEL_HEIGHT: usize, const N: usize, I, O, E> Filter
    for Convolution<KERNEL_WIDTH, KERNEL_HEIGHT, N, I, O, E>
where
    I: Pixel + IntoVector<N>,
    O: Pixel + FromVector<N>,
    E: ConstZero + Num + AsPrimitive<O::Element> + PartialOrd,
    I::Element: Copy + AsPrimitive<E>,
    O::Element: Copy + Bounded + Zero + AsPrimitive<E>,
{
    type Input = I;
    type Output = O;

    fn apply_pixel(
        &self,
        image: &Image<Self::Input>,
        x: u32,
        y: u32,
        _pixel: Self::Input,
    ) -> Self::Output {
        impl_convolution_apply_pixel! {
            self, image, x, y;
            Self::center(), KERNEL_WIDTH, KERNEL_HEIGHT
        }
    }
}

/// A convolution filter where the kernel matrix is dynamically allocated.
///
/// This is useful for cases where the kernel size is not known at compile time or when it needs
/// to be modified at runtime.
///
/// Note that for the lack of a dynamic vector type, this filter uses a matrix of scalar weights
/// as opposed to vector weights like in [`Convolution`].
#[derive(Clone, Debug)]
pub struct DynamicConvolution<const N: usize, Input, Output = Input, Element = f64>
where
    Element: Float,
    Input: Pixel + IntoVector<N>,
    Output: Pixel + FromVector<N>,
{
    /// The kernel, which is the flattened matrix of weights to apply to the output pixel.
    pub kernel: Vec<Element>,
    /// The width of the kernel.
    ///
    /// This is the number of columns in the kernel matrix.
    pub width: usize,
    _marker: PhantomData<(Input, Output)>,
}

impl<const N: usize, I, O, E> DynamicConvolution<N, I, O, E>
where
    I: Pixel + IntoVector<N>,
    O: Pixel + FromVector<N>,
    E: Float + Copy,
{
    /// Creates a new dynamic convolution filter with the given kernel.
    #[must_use]
    pub fn new(kernel: Vec<E>, width: usize) -> Self {
        assert!(!kernel.is_empty(), "Kernel cannot be empty");
        assert!(width > 0, "Kernel width must be greater than zero");
        assert_eq!(
            kernel.len() % width,
            0,
            "Kernel length must be a multiple of `width`"
        );

        Self {
            kernel,
            width,
            _marker: PhantomData,
        }
    }

    /// The dimensions of the kernel matrix.
    #[must_use]
    pub fn dimensions(&self) -> (usize, usize) {
        let height = self.kernel.len() / self.width;
        (self.width, height)
    }

    /// Returns the center indices of the kernel, corresponding to the pixel being processed.
    #[must_use]
    pub fn center(&self) -> (usize, usize) {
        let (height, width) = self.dimensions();
        (height / 2, width / 2)
    }

    /// Returns the weight at the given position in the kernel matrix, K_{i,j}.
    ///
    /// # Panics
    /// Panics if the indices are out of bounds.
    #[must_use]
    pub fn weight(&self, i: usize, j: usize) -> E {
        let (height, width) = self.dimensions();
        assert!(i < height && j < width, "Indices out of bounds");
        self.kernel[i * width + j]
    }

    /// Sets the weight at the given position in the kernel matrix, K_{i,j}.
    ///
    /// # Panics
    /// Panics if the indices are out of bounds.
    pub fn set_weight(&mut self, i: usize, j: usize, value: E) {
        let (height, width) = self.dimensions();
        assert!(i < height && j < width, "Indices out of bounds");
        self.kernel[i * width + j] = value;
    }

    /// Normalizes the kernel by dividing each element by the sum of all elements in the kernel.
    /// This is useful for ensuring that the output pixel values are within a certain range.
    pub fn normalize(&mut self)
    where
        E: Zero + Add<Output = E> + Div<Output = E> + Copy + PartialOrd,
    {
        let sum: E = self
            .kernel
            .iter()
            .copied()
            .fold(E::zero(), |acc, x| acc + x);
        if sum != E::zero() {
            for weight in &mut self.kernel {
                *weight = *weight / sum;
            }
        }
    }
}

impl<const N: usize, I, O, E> Filter for DynamicConvolution<N, I, O, E>
where
    I: Pixel + IntoVector<N>,
    O: Pixel + FromVector<N>,
    E: Float + ConstZero + Add<Output = E> + Mul<Output = E> + AsPrimitive<O::Element> + PartialOrd,
    I::Element: Copy + AsPrimitive<E>,
    O::Element: Bounded + Zero + AsPrimitive<E> + 'static,
{
    type Input = I;
    type Output = O;

    fn apply_pixel(
        &self,
        image: &Image<Self::Input>,
        x: u32,
        y: u32,
        _pixel: Self::Input,
    ) -> Self::Output {
        impl_convolution_apply_pixel! {
            self, image, x, y;
            self.center(), self.width, self.dimensions().1
        }
    }
}
