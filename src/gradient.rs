//! Handles rendering and logic of gradients.

#![allow(clippy::cast_lossless, clippy::cast_precision_loss)]

use crate::fill::{BoundingBox, Fill, IntoFill};
use crate::{Pixel, Rgba};

pub use colorgrad::{BlendMode, Interpolation};
use std::marker::PhantomData;

/// Checks if the gradient is safe to call [`normalize_positions`].
fn check_positions<P: Pixel>(colors: &[(P, f64)]) {
    assert!(!colors.is_empty(), "gradient must have at least one color");

    let mut last_known = 0.0;
    for (_, pos) in colors {
        if pos.is_nan() {
            continue;
        }

        assert!(
            *pos >= last_known,
            "position {pos} is less than the last known position {last_known}",
        );
        last_known = *pos;
    }
}

/// # Safety
/// The preconditions below must be met:
/// * Known colors of `colors` must be sorted by position.
/// * `colors` must not be empty.
unsafe fn normalize_positions<P: Pixel>(colors: &mut [(P, f64)]) {
    // If the first position is nan, it will be normalized to 0.0.
    if colors.get_unchecked(0).1.is_nan() {
        colors[0].1 = 0.0;
    }

    // If the last position is nan, it will be normalized to 1.0.
    if colors.last().unwrap_unchecked().1.is_nan() {
        colors.last_mut().unwrap_unchecked().1 = 1.0;
    }

    let mut i = 0;
    loop {
        if i == colors.len() - 1 {
            break;
        }

        let position = colors.get_unchecked(i).1;
        let peek = colors.get_unchecked(i + 1).1;
        if !peek.is_nan() {
            i += 1;
            continue;
        }

        // Count the number of nan positions until the next known position.
        let start = i;
        let mut count = 1;
        let mut next_position;
        loop {
            next_position = colors.get_unchecked(start + count).1;
            if !next_position.is_nan() {
                break;
            }
            count += 1;
            i += 1;
        }

        let increment = (next_position - position) / count as f64;
        for j in 1..count {
            colors.get_unchecked_mut(start + j).1 = increment.mul_add(j as f64, position);
        }

        i += 1;
    }
}

fn into_colorgrad<P: Pixel>(
    mut colors: Vec<(P, f64)>,
    interpolation: Interpolation,
    blend_mode: BlendMode,
) -> colorgrad::CustomGradient {
    check_positions(&colors);
    // SAFETY: The preconditions are met.
    unsafe { normalize_positions(&mut colors) };

    let (colors, positions): (Vec<_>, Vec<_>) = colors
        .into_iter()
        .map(|(color, position)| {
            let Rgba { r, g, b, a } = color.as_rgba();
            (colorgrad::Color::from_rgba8(r, g, b, a), position)
        })
        .unzip();

    let mut gradient = colorgrad::CustomGradient::new();
    gradient
        .colors(&colors)
        .domain(&positions)
        .interpolation(interpolation)
        .mode(blend_mode);

    gradient
}

/// A linear gradient.
///
/// # Example
/// ```
/// use ril::colors::{RED, BLUE};
///
/// # use ril::prelude::*;
/// # fn main() {
/// let mut image = Image::new(256, 256, Rgb::black());
/// let gradient = LinearGradient::new()
///     .with_angle_degrees(45.0)
///     .with_colors([RED, BLUE]);
///
/// image.draw(&Rectangle::from_bounding_box(64, 64, 192, 192).with_fill(gradient));
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct LinearGradient<P: Pixel> {
    /// The angle of the gradient in radians. Defaults to 0 radians. Angles outside the range
    /// `[0, 2 * PI)` will be normalized.
    pub angle: f64,
    /// A `Vec` of colors and their positions in the gradient, represented as `(color, position)`
    /// where `position` is a value in the range [0.0, 1.0].
    ///
    /// # Normalization of positions
    /// During building of this struct, there might be some positions that are `nan` which represent
    /// positions that will be normalized later. For example, `[0.0, nan, 1.0]` is normalized to
    /// `[0.0, 0.5, 1.0]` because `0.5` is the midpoint between `0.0` and `1.0`.
    ///
    /// Similarly, `[0.0, nan, nan, nan, 1.0]` is normalized to `[0.0, 0.25, 0.5, 0.75, 1.0]`
    /// because they evenly distribute between `0.0` and `1.0`.
    ///
    /// ## Normalization of endpoints
    /// If the first position is `nan`, it will be normalized to `0.0`. If the last position is
    /// `nan`, it will be normalized to `1.0`.
    pub colors: Vec<(P, f64)>,
    /// The interpolation mode to use when rendering the gradient. Defaults to
    /// [`Interpolation::Linear`].
    pub interpolation: Interpolation,
    /// The blending mode to use when rendering the gradient. Defaults to
    /// [`BlendMode::LinearRgb`]. If the gradient looks off or some colors are weirdly balanced,
    /// trying different blend modes here could help.
    pub blend_mode: BlendMode,
}

impl<P: Pixel> Default for LinearGradient<P> {
    fn default() -> Self {
        Self {
            angle: 0.0,
            colors: Vec::new(),
            interpolation: Interpolation::Linear,
            blend_mode: BlendMode::LinearRgb,
        }
    }
}

macro_rules! gradient_methods {
    () => {
        /// Sets the interpolation mode to use when rendering the gradient.
        #[must_use]
        pub const fn with_interpolation(mut self, interpolation: Interpolation) -> Self {
            self.interpolation = interpolation;
            self
        }

        /// Sets the blending mode to use when rendering the gradient.
        #[must_use]
        pub const fn with_blend_mode(mut self, blend_mode: BlendMode) -> Self {
            self.blend_mode = blend_mode;
            self
        }

        /// Sets the start color of the gradient. This will be rendered at the position `0.0`.
        ///
        /// # Note
        /// This uses `insert` instead of `push` to ensure that the start color is always at the
        /// beginning of the gradient. This means other colors will be shifted to the right.
        #[must_use]
        pub fn with_start_color(mut self, color: P) -> Self {
            self.colors.insert(0, (color, 0.0));
            self
        }

        /// Sets the end color of the gradient. This will be rendered at the position `1.0`.
        #[must_use]
        pub fn with_end_color(mut self, color: P) -> Self {
            self.colors.push((color, 1.0));
            self
        }

        /// Adds a color to the gradient at the specified position in place.
        ///
        /// # Panics
        /// * If the position is outside of the range `[0.0, 1.0]`. For auto-normalized positions, see
        ///   [`Self::push_color`].
        pub fn push_color_at(&mut self, position: f64, color: P) {
            assert!(
                (0.0..=1.0).contains(&position),
                "position must be in the range [0.0, 1.0]"
            );
            self.colors.push((color, position));
        }

        /// Takes this gradient and adds a color to the gradient at the specified position.
        ///
        /// # Panics
        /// * If the position is outside of the range `[0.0, 1.0]`. For auto-normalized positions, see
        ///   [`Self::with_color`].
        #[must_use]
        pub fn with_color_at(mut self, position: f64, color: P) -> Self {
            self.push_color_at(position, color);
            self
        }

        /// Adds a color to the gradient, automatically calculating its position. See the documentation
        /// for [`Self.colors`] for more information of how colors are normalized.
        ///
        /// # See Also
        /// * [`Self::push_color_at`] for adding a color at a specific position.
        pub fn push_color(&mut self, color: P) {
            self.colors.push((color, f64::NAN));
        }

        /// Takes this gradient and adds a color to the gradient, automatically calculating its position.
        /// See the documentation for [`Self.colors`] for more information of how colors are normalized.
        ///
        /// # See Also
        /// * [`Self::with_color_at`] for adding a color at a specific position.
        #[must_use]
        pub fn with_color(mut self, color: P) -> Self {
            self.push_color(color);
            self
        }

        /// Extends the colors and positions of this gradient with those specified in the given
        /// iterator of tuples represented as `(color, position)`.
        ///
        /// # Panics
        /// * If any of the positions are outside of the range `[0.0, 1.0]`. For auto-normalized
        ///   positions, see [`Self::extend`].
        pub fn extend_with_positions<I: IntoIterator<Item = (P, f64)>>(&mut self, iter: I) {
            self.colors.extend(iter);
        }

        /// Takes this gradient and extends it with the colors and positions specified in the
        /// given iterator of tuples represented as `(color, position)`.
        ///
        /// # Panics
        /// * If any of the positions are outside of the range `[0.0, 1.0]`. For auto-normalized
        ///   positions, see [`Self::with_colors`].
        pub fn with_colors_and_positions<I: IntoIterator<Item = (P, f64)>>(
            mut self,
            iter: I,
        ) -> Self {
            self.extend_with_positions(iter);
            self
        }

        /// Extends the colors of this gradient with those specified in the given iterator.
        /// The positions of the colors will be automatically calculated. See the documentation for
        /// [`Self.colors`] for more information of how colors are normalized.
        ///
        /// # See Also
        /// * [`Self::extend_with_positions`] for adding colors at specific positions.
        pub fn extend<I: IntoIterator<Item = P>>(&mut self, iter: I) {
            self.colors
                .extend(iter.into_iter().map(|color| (color, f64::NAN)));
        }

        /// Takes this gradient and extends it with the colors specified in the given iterator.
        #[must_use]
        pub fn with_colors<I: IntoIterator<Item = P>>(mut self, iter: I) -> Self {
            self.extend(iter);
            self
        }
    };
}

impl<P: Pixel> LinearGradient<P> {
    /// Creates a new [`LinearGradient`] with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the angle of the gradient in **radians**. Angles outside of the range `[0.0, 2 * PI)`
    /// will be normalized.
    ///
    /// If your angle is in degrees, the [`f64::to_radians`] method can be used to convert into
    /// degrees, or the convenience method [`Self::with_angle_degrees`] can be used.
    #[must_use]
    pub const fn with_angle(mut self, angle: f64) -> Self {
        self.angle = angle;
        self
    }

    /// A shortcut method to set the angle of the gradient in **degrees**. Angles outside of the
    /// range `[0.0, 360.0)` will be normalized.
    ///
    /// See [`Self::with_angle`] for more information.
    #[must_use]
    pub fn with_angle_degrees(self, angle: f64) -> Self {
        self.with_angle(angle.to_radians())
    }

    gradient_methods!();
}

impl<P: Pixel> IntoFill for LinearGradient<P> {
    type Pixel = P;
    type Fill = LinearGradientFill<Self::Pixel>;

    fn into_fill(mut self) -> Self::Fill {
        self.angle = self.angle.rem_euclid(std::f64::consts::TAU);
        let (ty, tx) = self.angle.sin_cos();
        let clone_gradient = into_colorgrad(self.colors, self.interpolation, self.blend_mode);

        LinearGradientFill {
            x: 0.0,
            y: 0.0,
            tx,
            ty,
            width: 0.0,
            height: 0.0,
            half_width: 0.0,
            half_height: 0.0,
            // SAFETY: validated by `check_positions` and `normalize_positions`.
            gradient: unsafe { clone_gradient.build().unwrap_unchecked() },
            clone_gradient,
            _marker: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct LinearGradientFill<P: Pixel> {
    x: f64,
    y: f64,
    tx: f64,
    ty: f64,
    width: f64,
    height: f64,
    half_width: f64,
    half_height: f64,
    pub(crate) gradient: colorgrad::Gradient,
    clone_gradient: colorgrad::CustomGradient,
    _marker: PhantomData<P>,
}

macro_rules! gradient_clone {
    ($self:ident: $($items:ident),+) => {{
        Self {
            $($items: $self.$items,)+
            gradient: $self.clone_gradient.build().unwrap(),
            clone_gradient: $self.clone_gradient.clone(),
            _marker: PhantomData,
        }
    }};
}

// We can't derive `Clone` because `colorgrad::Gradient` doesn't implement `Clone`.
impl<P: Pixel> Clone for LinearGradientFill<P> {
    fn clone(&self) -> Self {
        gradient_clone!(self: x, y, tx, ty, width, height, half_width, half_height)
    }
}

impl<P: Pixel> Fill<P> for LinearGradientFill<P> {
    fn set_bounding_box(&mut self, (x1, y1, x2, y2): BoundingBox<u32>) {
        let width = (x2 - x1) as f64;
        let height = (y2 - y1) as f64;

        self.x = x1 as f64;
        self.y = y1 as f64;
        self.width = width;
        self.height = height;
        self.half_width = width / 2.0;
        self.half_height = height / 2.0;
    }

    fn get_pixel(&self, x: u32, y: u32) -> P {
        // Make the coordinates relative to the center of the bounding box.
        let x = x as f64 - self.half_width - self.x;
        let y = y as f64 - self.half_height - self.y;

        // Calculate the dot product of the position vector and the angle vector.
        let t = (x / self.width).mul_add(self.tx, (y / self.height) * self.ty);

        // Get the color from the gradient.
        let (r, g, b, a) = self.gradient.at(0.5 + t).to_linear_rgba_u8();
        P::from_raw_parts(crate::ColorType::Rgba, 8, &[r, g, b, a]).unwrap()
    }
}

/// Represents where the center of a radial or conic gradient is placed.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum GradientPosition {
    /// A pair of coordinates relative to the shape rendered, where `0.0` is the left-most/top-most
    /// coordinate of the bounding box and `1.0` is the right-most/bottom-most coordinate of the
    /// bounding box. For example `Relative(0.5, 0.5)` indicates the center.
    Relative(f64, f64),
    /// A pair of absolute coordinates with accordance to the canvas (and not the shape itself).
    Absolute(u32, u32),
}

impl GradientPosition {
    /// A shorthand for `Relative(0.5, 0.5)`.
    pub const CENTER: Self = Self::Relative(0.5, 0.5);
}

impl Default for GradientPosition {
    fn default() -> Self {
        Self::CENTER
    }
}

/// How a radial gradient should cover its shape if the aspect ratio of the bounding box != 1.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RadialGradientCover {
    /// Stretch the gradient. This is the default behavior.
    Stretch,
    /// Set the color stop at `1.0` to render at the end of the shortest side, in a way that shows
    /// the entire gradient.
    Shortest,
    /// Set the color stop at `1.0` to render at the end of the longest side, in a way that cuts
    /// off the gradient.
    Longest,
}

impl Default for RadialGradientCover {
    fn default() -> Self {
        Self::Stretch
    }
}

/// A radial gradient.
#[derive(Clone, Debug)]
pub struct RadialGradient<P: Pixel> {
    /// The position of the center of the radial gradient (where the radial gradient "radiates"
    /// from).
    pub position: GradientPosition,
    /// How the gradient should cover the bounding box.
    pub cover: RadialGradientCover,
    /// A `Vec` of colors and their positions in the gradient, represented as `(color, position)`
    /// where `position` is a value in the range [0.0, 1.0].
    ///
    /// # Normalization of positions
    /// During building of this struct, there might be some positions that are `nan` which represent
    /// positions that will be normalized later. For example, `[0.0, nan, 1.0]` is normalized to
    /// `[0.0, 0.5, 1.0]` because `0.5` is the midpoint between `0.0` and `1.0`.
    ///
    /// Similarly, `[0.0, nan, nan, nan, 1.0]` is normalized to `[0.0, 0.25, 0.5, 0.75, 1.0]`
    /// because they evenly distribute between `0.0` and `1.0`.
    ///
    /// ## Normalization of endpoints
    /// If the first position is `nan`, it will be normalized to `0.0`. If the last position is
    /// `nan`, it will be normalized to `1.0`.
    pub colors: Vec<(P, f64)>,
    /// The interpolation mode to use when rendering the gradient. Defaults to
    /// [`Interpolation::Linear`].
    pub interpolation: Interpolation,
    /// The blending mode to use when rendering the gradient. Defaults to
    /// [`BlendMode::LinearRgb`]. If the gradient looks off or some colors are weirdly balanced,
    /// trying different blend modes here could help.
    pub blend_mode: BlendMode,
}

impl<P: Pixel> Default for RadialGradient<P> {
    fn default() -> Self {
        Self {
            position: GradientPosition::CENTER,
            cover: RadialGradientCover::Stretch,
            colors: Vec::new(),
            interpolation: Interpolation::Linear,
            blend_mode: BlendMode::LinearRgb,
        }
    }
}

impl<P: Pixel> RadialGradient<P> {
    /// Creates a new empty radial gradient.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the position of the center of the radial gradient (where the radial gradient "radiates"
    /// from).
    #[must_use]
    pub const fn with_position(mut self, position: GradientPosition) -> Self {
        self.position = position;
        self
    }

    /// Sets how the gradient should cover the bounding box.
    #[must_use]
    pub const fn with_cover(mut self, cover: RadialGradientCover) -> Self {
        self.cover = cover;
        self
    }

    gradient_methods!();
}

impl<P: Pixel> IntoFill for RadialGradient<P> {
    type Pixel = P;
    type Fill = RadialGradientFill<Self::Pixel>;

    fn into_fill(self) -> Self::Fill {
        let clone_gradient = into_colorgrad(self.colors, self.interpolation, self.blend_mode);
        let (cx, cy) = match self.position {
            GradientPosition::Absolute(x, y) => (x as f64, y as f64),
            GradientPosition::Relative(..) => (0.0, 0.0),
        };

        RadialGradientFill {
            cx,
            cy,
            dist: 0.0,
            ratio: 0.0,
            position: self.position,
            cover: self.cover,
            // SAFETY: validated by `check_positions` and `normalize_positions`.
            gradient: unsafe { clone_gradient.build().unwrap_unchecked() },
            clone_gradient,
            _marker: PhantomData,
        }
    }
}

pub struct RadialGradientFill<P: Pixel> {
    cx: f64,
    cy: f64,
    dist: f64,
    ratio: f64,
    position: GradientPosition,
    cover: RadialGradientCover,
    pub(crate) gradient: colorgrad::Gradient,
    clone_gradient: colorgrad::CustomGradient,
    _marker: PhantomData<P>,
}

// We can't derive `Clone` because `colorgrad::Gradient` doesn't implement `Clone`.
impl<P: Pixel> Clone for RadialGradientFill<P> {
    fn clone(&self) -> Self {
        gradient_clone!(self: cx, cy, dist, ratio, position, cover)
    }
}

impl<P: Pixel> Fill<P> for RadialGradientFill<P> {
    fn set_bounding_box(&mut self, (x1, y1, x2, y2): BoundingBox<u32>) {
        let width = (x2 - x1) as f64;
        let height = (y2 - y1) as f64;

        self.dist = match self.cover {
            RadialGradientCover::Stretch | RadialGradientCover::Shortest => {
                if matches!(self.cover, RadialGradientCover::Stretch) {
                    self.ratio = height / width;
                }
                if width < height {
                    width
                } else {
                    height
                }
            }
            RadialGradientCover::Longest => {
                if width > height {
                    width
                } else {
                    height
                }
            }
        };

        if let GradientPosition::Relative(x, y) = self.position {
            self.cx = x.mul_add(width, x1 as f64);
            self.cy = y.mul_add(height, y1 as f64);
        }
    }

    fn get_pixel(&self, x: u32, y: u32) -> P {
        let dx = x as f64 - self.cx;
        let dy = y as f64 - self.cy;

        let dist = match self.cover {
            RadialGradientCover::Stretch => {
                if self.ratio < 1.0 {
                    dy.hypot(dx * self.ratio)
                } else {
                    dx.hypot(dy / self.ratio)
                }
            }
            _ => dx.hypot(dy),
        };

        // Get the color from the gradient
        let (r, g, b, a) = self.gradient.at(dist / self.dist).to_linear_rgba_u8();
        P::from_raw_parts(crate::ColorType::Rgba, 8, &[r, g, b, a]).unwrap()
    }
}

/// A conic gradient.
#[derive(Clone, Debug)]
pub struct ConicGradient<P: Pixel> {
    /// The angle of the conic gradient, in radians. Defaults to `0.0`, where the start and end
    /// values will meet vertically at the top.
    pub angle: f64,
    /// The position of the center of the conic gradient. Defaults to the center of the bounding
    /// box.
    pub position: GradientPosition,
    /// A `Vec` of colors and their positions in the gradient, represented as `(color, position)`
    /// where `position` is a value in the range [0.0, 1.0].
    ///
    /// # Normalization of positions
    /// During building of this struct, there might be some positions that are `nan` which represent
    /// positions that will be normalized later. For example, `[0.0, nan, 1.0]` is normalized to
    /// `[0.0, 0.5, 1.0]` because `0.5` is the midpoint between `0.0` and `1.0`.
    ///
    /// Similarly, `[0.0, nan, nan, nan, 1.0]` is normalized to `[0.0, 0.25, 0.5, 0.75, 1.0]`
    /// because they evenly distribute between `0.0` and `1.0`.
    ///
    /// ## Normalization of endpoints
    /// If the first position is `nan`, it will be normalized to `0.0`. If the last position is
    /// `nan`, it will be normalized to `1.0`.
    pub colors: Vec<(P, f64)>,
    /// The interpolation mode to use when rendering the gradient. Defaults to
    /// [`Interpolation::Linear`].
    pub interpolation: Interpolation,
    /// The blending mode to use when rendering the gradient. Defaults to
    /// [`BlendMode::LinearRgb`]. If the gradient looks off or some colors are weirdly balanced,
    /// trying different blend modes here could help.
    pub blend_mode: BlendMode,
}

impl<P: Pixel> Default for ConicGradient<P> {
    fn default() -> Self {
        Self {
            angle: 0.0,
            position: GradientPosition::CENTER,
            colors: Vec::new(),
            interpolation: Interpolation::Linear,
            blend_mode: BlendMode::LinearRgb,
        }
    }
}

impl<P: Pixel> ConicGradient<P> {
    /// Creates a new conic gradient.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the angle of the gradient in **radians**. Angles outside of the range `[0.0, 2 * PI)`
    /// will be normalized.
    ///
    /// If your angle is in degrees, the [`f64::to_radians`] method can be used to convert into
    /// degrees, or the convenience method [`Self::with_angle_degrees`] can be used.
    #[must_use]
    pub const fn with_angle(mut self, angle: f64) -> Self {
        self.angle = angle;
        self
    }

    /// A shortcut method to set the angle of the gradient in **degrees**. Angles outside of the
    /// range `[0.0, 360.0)` will be normalized.
    ///
    /// See [`Self::with_angle`] for more information.
    #[must_use]
    pub fn with_angle_degrees(self, angle: f64) -> Self {
        self.with_angle(angle.to_radians())
    }

    /// Sets the position of the center of the gradient.
    #[must_use]
    pub const fn with_position(mut self, position: GradientPosition) -> Self {
        self.position = position;
        self
    }

    gradient_methods!();
}

impl<P: Pixel> IntoFill for ConicGradient<P> {
    type Pixel = P;
    type Fill = ConicGradientFill<Self::Pixel>;

    fn into_fill(self) -> Self::Fill {
        let clone_gradient = into_colorgrad(self.colors, self.interpolation, self.blend_mode);
        let (cx, cy) = match self.position {
            GradientPosition::Absolute(x, y) => (x as f64, y as f64),
            GradientPosition::Relative(..) => (0.0, 0.0),
        };

        ConicGradientFill {
            cx,
            cy,
            angle: self.angle,
            position: self.position,
            // SAFETY: validated by `check_positions` and `normalize_positions`.
            gradient: unsafe { clone_gradient.build().unwrap_unchecked() },
            clone_gradient,
            _marker: PhantomData,
        }
    }
}

/// A conic gradient fill.
#[derive(Debug)]
pub struct ConicGradientFill<P: Pixel> {
    cx: f64,
    cy: f64,
    angle: f64,
    position: GradientPosition,
    gradient: colorgrad::Gradient,
    clone_gradient: colorgrad::CustomGradient,
    _marker: PhantomData<P>,
}

impl<P: Pixel> Clone for ConicGradientFill<P> {
    fn clone(&self) -> Self {
        gradient_clone!(self: cx, cy, angle, position)
    }
}

impl<P: Pixel> Fill<P> for ConicGradientFill<P> {
    fn set_bounding_box(&mut self, (x1, y1, x2, y2): BoundingBox<u32>) {
        if let GradientPosition::Relative(x, y) = self.position {
            let x1 = x1 as f64;
            let y1 = y1 as f64;

            self.cx = x.mul_add(x2 as f64 - x1, x1);
            self.cy = y.mul_add(y2 as f64 - y1, y1);
        }
    }

    fn get_pixel(&self, x: u32, y: u32) -> P {
        let mut angle = (x as f64 - self.cx).atan2(y as f64 - self.cy) - self.angle;
        angle /= std::f64::consts::TAU;

        // Get the color from the gradient
        let (r, g, b, a) = self.gradient.at(angle + 0.5).to_linear_rgba_u8();
        P::from_raw_parts(crate::ColorType::Rgba, 8, &[r, g, b, a]).unwrap()
    }
}
