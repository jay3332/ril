//! Handles rendering and logic of gradients.

use crate::{Pixel, Rgba};
use std::marker::PhantomData;

pub use colorgrad::{BlendMode, Interpolation};

pub(crate) trait IntoFill<P: Pixel> {
    type Fill: Fill<P>;

    fn into_fill(self) -> Self::Fill;
}

#[cfg_attr(feature = "const-pixels", const_trait)]
pub(crate) trait Fill<P: Pixel> {
    fn get_pixel(&self, bounding_box: &(u32, u32, u32, u32), x: u32, y: u32) -> P;
}

impl<P: Pixel> IntoFill<P> for P {
    type Fill = SolidFill<P>;

    fn into_fill(self) -> Self::Fill {
        SolidFill(self)
    }
}

pub(crate) struct SolidFill<P: Pixel>(P);

#[cfg(feature = "const-pixels")]
impl<P: Pixel> const Fill<P> for SolidFill<P> {
    fn get_pixel(&self, _: &(u32, u32, u32, u32), _: u32, _: u32) -> P {
        self.0
    }
}

#[cfg(not(feature = "const-pixels"))]
impl<P: Pixel> Fill<P> for SolidFill<P> {
    fn get_pixel(&self, _: &(u32, u32, u32, u32), _: u32, _: u32) -> P {
        self.0
    }
}

/// A linear gradient.
#[derive(Clone)]
pub struct LinearGradient<P: Pixel> {
    /// The angle of the gradient in radians. Defaults to 0 radians.  Angles outside of the range
    /// `[0, 2 * PI)` will be normalized.
    pub angle: f64,
    /// A `Vec` of colors and their positions in the gradient, represented as `(color, position)`
    /// where `position` is a value in the range [0.0, 1.0].
    ///
    /// # Normalization of positions
    /// During building of this struct, there might be some positions of `-1.0`, these represent
    /// positions that will be normalized later. For example, `[0.0, -1.0, 1.0]` is normalized to
    /// `[0.0, 0.5, 1.0]` because `0.5` is the midpoint between `0.0` and `1.0`.
    ///
    /// Similarly, `[0.0, -1.0, -1.0, -1.0, 1.0]` is normalized to `[0.0, 0.25, 0.5, 0.75, 1.0]`
    /// because they evenly distribute between `0.0` and `1.0`.
    ///
    /// ## Normalization of endpoints
    /// If the first position is `-1.0`, it will be normalized to `0.0`. If the last position is
    /// `-1.0`, it will be normalized to `1.0`.
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
    /// degrees.
    #[must_use]
    pub const fn with_angle(mut self, angle: f64) -> Self {
        self.angle = angle;
        self
    }

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
    /// [`Self::push_color`].
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
    /// [`Self::with_color`].
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
        self.colors.push((color, -1.0));
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
    /// positions, see [`Self::extend`].
    pub fn extend_with_positions<I: IntoIterator<Item = (P, f64)>>(&mut self, iter: I) {
        self.colors.extend(iter);
    }

    /// Extends the colors of this gradient with those specified in the given iterator.
    /// The positions of the colors will be automatically calculated. See the documentation for
    /// [`Self.colors`] for more information of how colors are normalized.
    ///
    /// # See Also
    /// * [`Self::extend_with_positions`] for adding colors at specific positions.
    pub fn extend<I: IntoIterator<Item = P>>(&mut self, iter: I) {
        self.colors
            .extend(iter.into_iter().map(|color| (color, -1.0)));
    }

    /// Checks if the gradient is safe to call [`Self::normalize_positions`].
    fn check_positions(&mut self) {
        assert!(
            !self.colors.is_empty(),
            "gradient must have at least one color"
        );

        let mut last_known = 0.0;
        for (_, pos) in self.colors.iter() {
            if *pos == -1.0 {
                continue;
            }

            assert!(
                *pos >= last_known,
                "position {} is less than the last known position {}",
                pos,
                last_known
            );
            last_known = *pos;
        }
    }

    /// # Safety
    /// The preconditions below must be met:
    /// * Known colors of `self.colors` must be sorted by position.
    /// * `self.colors` must not be empty.
    unsafe fn normalize_positions(&mut self) {
        // If the first position is -1.0, it will be normalized to 0.0.
        if self.colors.get_unchecked(0).1 == -1.0 {
            self.colors[0].1 = 0.0;
        }

        // If the last position is -1.0, it will be normalized to 1.0.
        if self.colors.last().unwrap_unchecked().1 == -1.0 {
            self.colors.last_mut().unwrap_unchecked().1 = 1.0;
        }

        let mut i = 0;
        loop {
            if i == self.colors.len() - 1 {
                break;
            }

            let position = self.colors.get_unchecked(i).1;
            let peek = self.colors.get_unchecked(i + 1).1;
            if peek != -1.0 {
                i += 1;
                continue;
            }

            // Count the number of -1.0 positions until the next known position.
            let start = i;
            let mut count = 1;
            let mut next_position;
            loop {
                next_position = self.colors.get_unchecked(start + count).1;
                if next_position != -1.0 {
                    break;
                }
                count += 1;
                i += 1;
            }

            let increment = (next_position - position) / count as f64;
            for j in 1..count {
                self.colors.get_unchecked_mut(start + j).1 = position + increment * j as f64;
            }

            i += 1;
        }
    }

    fn into_colorgrad(mut self) -> colorgrad::Gradient {
        self.check_positions();
        // SAFETY: The preconditions are met.
        unsafe { self.normalize_positions() };

        let (colors, positions): (Vec<_>, Vec<_>) = self
            .colors
            .into_iter()
            .map(|(color, position)| {
                let Rgba { r, g, b, a } = color.as_rgba();
                (colorgrad::Color::from_rgba8(r, g, b, a), position)
            })
            .unzip();

        // SAFETY: check_positions + normalize_positions guarantees a valid domain
        unsafe {
            colorgrad::CustomGradient::new()
                .colors(&colors)
                .domain(&positions)
                .interpolation(self.interpolation)
                .mode(self.blend_mode)
                .build()
                .unwrap_unchecked()
        }
    }
}

impl<P: Pixel> IntoFill<P> for LinearGradient<P> {
    type Fill = LinearGradientFill<P>;

    fn into_fill(mut self) -> Self::Fill {
        self.angle = self.angle.rem_euclid(std::f64::consts::TAU);

        LinearGradientFill {
            x: self.angle.cos(),
            y: self.angle.sin(),
            gradient: self.into_colorgrad(),
            _marker: PhantomData,
        }
    }
}

pub(crate) struct LinearGradientFill<P: Pixel> {
    x: f64,
    y: f64,
    gradient: colorgrad::Gradient,
    _marker: PhantomData<P>,
}

impl<P: Pixel> Fill<P> for LinearGradientFill<P> {
    fn get_pixel(&self, (x1, y1, x2, y2): &(u32, u32, u32, u32), x: u32, y: u32) -> P {
        let width = (x2 - x1) as f64;
        let height = (y2 - y1) as f64;

        // Make the coordinates relative to the center of the bounding box.
        let x = x as f64 - width / 2.0;
        let y = y as f64 - height / 2.0;

        // Calculate the dot product of the position vector and the angle vector.
        let t = (x / width) * self.x + (y / height) * self.y;

        // Get the color from the gradient.
        let (r, g, b, a) = self.gradient.at(0.5 + t).to_linear_rgba_u8();
        P::from_raw_parts(crate::ColorType::Rgba, 8, &[r, g, b, a]).unwrap()
    }
}
