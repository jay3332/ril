//! Encloses most drawing implementations and drawable objects.

use crate::{Image, OverlayMode, Pixel};
use std::ops::DerefMut;

/// A common trait for all objects able to be drawn on an image.
///
/// Whether or not to implement this trait is more or less a matter of semantics.
pub trait Draw<P: Pixel> {
    /// Draws the object to the given image.
    fn draw<I: DerefMut<Target = Image<P>>>(&self, image: I);
}

/// Represents whether a border is inset, outset, or if it lays in the center.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BorderPosition {
    /// An inset border. May overlap the contents of inside the shape.
    Inset,
    /// A border that is balanced between the inside and outside of the shape.
    Center,
    /// An outset border. May overlap the contents of outside the shape. This is the default
    /// behavior because it is usually what you would expect.
    Outset,
}

impl Default for BorderPosition {
    fn default() -> Self {
        Self::Outset
    }
}

/// Represents a shape border.
///
/// TODO: Add support for rounded borders
#[derive(Clone, Debug, Default)]
pub struct Border<P: Pixel> {
    /// The color of the border.
    pub color: P,
    /// The thickness of the border, in pixels.
    pub thickness: u32,
    /// The position of the border.
    pub position: BorderPosition,
}

impl<P: Pixel> Border<P> {
    /// todo!()
    ///
    /// # Panics
    /// * Panics if the border thickness is 0.
    pub fn new(color: P, thickness: u32) -> Self {
        assert_ne!(thickness, 0, "border thickness cannot be 0");

        Self {
            color,
            thickness,
            position: BorderPosition::default(),
        }
    }

    #[must_use]
    pub const fn with_color(mut self, color: P) -> Self {
        self.color = color;
        self
    }

    /// todo!()
    ///
    /// # Panics
    /// * Panics if the border thickness is 0.
    #[must_use]
    pub fn with_thickness(mut self, thickness: u32) -> Self {
        assert_ne!(thickness, 0, "border thickness cannot be 0");
        self.thickness = thickness;
        self
    }

    #[must_use]
    pub const fn with_position(mut self, position: BorderPosition) -> Self {
        self.position = position;
        self
    }

    // Bounds are inclusive
    const fn bounds(&self) -> (u32, u32, P) {
        let Self {
            color,
            thickness,
            position,
        } = self;
        let thickness = *thickness;

        let (inner, outer) = match position {
            BorderPosition::Outset => (0, thickness),
            BorderPosition::Inset => (thickness, 0),
            BorderPosition::Center => {
                let offset = thickness / 2;
                // This way, the two will still sum to offset
                (offset, thickness - offset)
            }
        };

        (inner, outer, *color)
    }
}

/// A rectangle.
///
/// Using any of the predefined construction methods will automatically set the position to
/// `(0, 0)`. If you want to specify a different position, use the `with_position` method.
///
/// # Note
/// You must specify a width and height for the rectangle with something such as [`with_size`].
/// If you don't, a panic will be raised during drawing. You can also try using
/// [`from_bounding_box`] to create a rectangle from a bounding box, which automatically fills
/// in the size.
///
/// Additionally, a panic will be raised during drawing if you do not specify either a fill color
/// or a border. these can be set with [`with_fill`] and [`with_border`] respectively.
#[derive(Clone, Debug, Default)]
pub struct Rectangle<P: Pixel> {
    /// The position of the rectangle. The top-left corner of the rectangle will be rendered at
    /// this position.
    pub position: (u32, u32),
    /// The dimensions of the rectangle, in pixels.
    pub size: (u32, u32),
    /// The border data of the rectangle, or None if there is no border.
    pub border: Option<Border<P>>,
    /// The fill color of the rectangle, or None if there is no fill.
    pub fill: Option<P>,
    /// The overlay mode of the rectangle, or None to inherit from the overlay mode of the image.
    pub overlay: Option<OverlayMode>,
}

impl<P: Pixel> Rectangle<P> {
    /// Creates a new rectangle with default values.
    ///
    /// This immediately sets the position to `(0, 0)`
    /// and you must explicitly set the size of the rectangle with [`with_size`] in order to set a
    /// size for the rectangle. If no size is set before drawing, you will receive a panic.
    ///
    /// This also does not set any border or fill for the rectangle, you must explicitly set either
    /// one of them with [`with_fill`] or [`with_border`] respectively or else you will receive a
    /// panic at draw-time.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new rectangle from two coordinates specified as 4 parameters.
    ///
    /// The first coordinate is the top-left corner of the rectangle, and the second coordinate is
    /// the bottom-right corner of the rectangle.
    ///
    /// # Panics
    /// * Panics if the bounding box is invalid.
    #[must_use]
    pub fn from_bounding_box(x1: u32, y1: u32, x2: u32, y2: u32) -> Self {
        assert!(x2 >= x1, "invalid bounding box");
        assert!(y2 >= y1, "invalid bounding box");

        Self::default()
            .with_position(x1, y1)
            .with_size(x2 - x1, y2 - y1)
    }

    /// Sets the position of the rectangle.
    #[must_use]
    pub const fn with_position(mut self, x: u32, y: u32) -> Self {
        self.position = (x, y);
        self
    }

    /// Sets the size of the rectangle in pixels.
    #[must_use]
    pub const fn with_size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    /// Sets the border information of the rectangle.
    ///
    /// # See Also
    /// * [`Border`]
    #[must_use]
    pub const fn with_border(mut self, border: Border<P>) -> Self {
        self.border = Some(border);
        self
    }

    /// Sets the fill color of the rectangle.
    #[must_use]
    pub const fn with_fill(mut self, fill: P) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Sets the overlay mode of the rectangle.
    #[must_use]
    pub const fn with_overlay_mode(mut self, mode: OverlayMode) -> Self {
        self.overlay = Some(mode);
        self
    }
}

impl<P: Pixel> Draw<P> for Rectangle<P> {
    fn draw<I: DerefMut<Target = Image<P>>>(&self, mut image: I) {
        assert!(
            self.fill.is_some() || self.border.is_some(),
            "must provide one of either fill or border, try calling .with_fill()"
        );
        assert!(
            self.size.0 > 0 || self.size.1 > 0,
            "rectangle must have a non-zero width and height, have you called .with_size() yet?"
        );

        let (x1, y1) = self.position;
        let (w, h) = self.size;
        // Exclusive bounds
        let (x2, y2) = (x1 + w, y1 + h);
        let overlay = self.overlay.unwrap_or(image.overlay);

        // Draw the fill first
        if let Some(fill) = self.fill {
            for y in y1..y2 {
                for x in x1..x2 {
                    image.overlay_pixel_with_mode(x, y, fill, overlay);
                }
            }
        }

        // Draw the border on top of the fill. If the border has alpha this could result in the
        // border blending with the fill, but this is rarely a problem. This behavior isn't really
        // normal though and I do plan to fix it, for example calculating border bounds first and
        // only filling in pixels that are not in those bounds.
        if let Some((inner, outer, color)) = self.border.as_ref().map(Border::bounds) {
            // Top and bottom border
            for y in (y1 - outer..y1 + inner).chain(y2 - inner..y2 + outer) {
                for x in x1..x2 {
                    image.overlay_pixel_with_mode(x, y, color, overlay);
                }
            }

            // Left and right border
            for x in (x1 - outer..x1 + inner).chain(x2 - inner..x2 + outer) {
                for y in y1 - outer..y2 + outer {
                    image.overlay_pixel_with_mode(x, y, color, overlay);
                }
            }
        }
    }
}

/// An ellipse, which could be a circle.
///
/// Using any of the predefined constructors will automatically set the position to `(0, 0)` and
/// you must explicitly set the size of the ellipse with [`with_size`] in order to set a size for
/// the ellipse. If no size is set before drawing, you will receive a panic.
///
/// This also does not set any border or fill for the ellipse, you must explicitly set either one
/// of them with [`with_fill`] or [`with_border`] respectively or else you will receive a panic at
/// draw-time.
#[derive(Clone, Debug, Default)]
pub struct Ellipse<P: Pixel> {
    /// The center position of the ellipse.
    /// The center of this ellipse will be rendered at this position.
    pub position: (u32, u32),
    /// The radii of the ellipse, in pixels; (horizontal, vertical).
    pub radii: (u32, u32),
    // The border data for the ellipse if any.
    pub border: Option<Border<P>>,
    // The fill color for the ellipse if any.
    pub fill: Option<P>,
    // The overlay mode for the ellipse or None to inherit from the image's overlay mode.
    pub overlay: Option<OverlayMode>,
}

impl<P: Pixel> Ellipse<P> {
    /// Creates a new ellipse.
    ///
    /// The ellipse by default will be centered at `(0, 0)` which will always cut off a portion
    /// of the ellipse. You should explicitly set the position of the center of the ellipse with
    /// [`with_position`] or else you will receive a panic at draw-time.
    ///
    /// You must also specify a size for the ellipse with [`with_size`] or else you will receive a
    /// panic at draw-time.
    ///
    /// Finally, you must also specify a fill color with [`with_fill`] or a border color with
    /// [`with_border`] or else you will receive a panic at draw-time.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new ellipse from the given bounding box.
    ///
    /// # Panics
    /// * `x2 < x1`
    /// * `y2 < y1`
    #[must_use]
    pub fn from_bounding_box(x1: u32, y1: u32, x2: u32, y2: u32) -> Self {
        assert!(x2 >= x1, "invalid bounding box");
        assert!(y2 >= y1, "invalid bounding box");

        let (dx, dy) = (x2 - x1, y2 - y1);
        let (x, y) = (x1 + dx / 2, y1 + dy / 2);

        Self::default().with_position(x, y).with_size(dx, dy)
    }

    /// Creates a new circle with the given center position and radius.
    #[must_use]
    pub fn circle(x: u32, y: u32, radius: u32) -> Self {
        Self::default()
            .with_position(x, y)
            .with_radii(radius, radius)
    }

    /// Sets the position of the ellipse.
    #[must_use]
    pub const fn with_position(mut self, x: u32, y: u32) -> Self {
        self.position = (x, y);
        self
    }

    /// Sets the radii of the ellipse in pixels.
    #[must_use]
    pub const fn with_radii(mut self, width: u32, height: u32) -> Self {
        self.radii = (width, height);
        self
    }

    /// Sets the diameters of the ellipse in pixels.
    #[must_use]
    pub const fn with_size(mut self, width: u32, height: u32) -> Self {
        self.radii = (width / 2, height / 2);
        self
    }

    /// Sets the border of the ellipse.
    #[must_use]
    pub const fn with_border(mut self, border: Border<P>) -> Self {
        self.border = Some(border);
        self
    }

    /// Sets the fill color of the ellipse.
    #[must_use]
    pub const fn with_fill(mut self, fill: P) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Sets the overlay mode of the ellipse.
    #[must_use]
    pub const fn with_overlay_mode(mut self, mode: OverlayMode) -> Self {
        self.overlay = Some(mode);
        self
    }

    // Used when there is no border
    #[allow(clippy::cast_possible_wrap)]
    fn rasterize_filled_circle<I: DerefMut<Target = Image<P>>>(&self, mut image: I) {
        let radius = self.radii.0 as i32;

        let mut x = 0;
        let mut y = radius;
        let mut p = 1 - radius;

        let (h, k) = self.position;
        let (h, k) = (h as i32, k as i32);

        #[allow(unused_variables)] // rust knows this, but the external linter doesn't
        let fill = self.fill.unwrap();
        #[allow(unused_variables)] // rust knows this, but the external linter doesn't
        let overlay = self.overlay.unwrap_or(image.overlay);

        macro_rules! line {
            ($from:expr, $to:expr, $y:expr) => {{
                let y = ($y) as u32;

                for x in ($from)..=($to) {
                    image.overlay_pixel_with_mode(x as u32, y, fill, overlay);
                }
            }};
        }

        while x <= y {
            line!(h - x, h + x, k + y);
            line!(h - y, h + y, k + x);
            line!(h - x, h + x, k - y);
            line!(h - y, h + y, k - x);

            x += 1;
            if p < 0 {
                p += 2 * x + 1;
            } else {
                y -= 1;
                p += 2 * (x - y) + 1;
            }
        }
    }

    // Used when there is no border
    #[allow(clippy::cast_possible_wrap, clippy::cast_precision_loss)]
    fn rasterize_filled_ellipse<I: DerefMut<Target = Image<P>>>(&self, mut image: I) {
        let (ch, k) = self.position;
        #[allow(unused_variables)] // rust knows this, but the external linter doesn't
        let (ch, k) = (ch as i32, k as i32);

        let (w, h) = self.radii;
        let (w, h) = (w as i32, h as i32);
        let (w2, h2) = (w * w, h * h);

        let mut x = 0;
        let mut y = h;
        let mut px = 0;
        let mut py = 2 * w2 * y;

        #[allow(unused_variables)] // rust knows this, but the external linter doesn't
        let fill = self.fill.unwrap();
        #[allow(unused_variables)] // rust knows this, but the external linter doesn't
        let overlay = self.overlay.unwrap_or(image.overlay);

        macro_rules! line {
            ($from:expr, $to:expr, $y:expr) => {{
                let y = ($y) as u32;

                for x in ($from)..=($to) {
                    image.overlay_pixel_with_mode(x as u32, y, fill, overlay);
                }
            }};
            ($x:expr, $y:expr) => {{
                line!(ch - $x, ch + $x, k + $y);
                line!(ch - $x, ch + $x, k - $y);
            }};
        }

        let mut p = 0.25_f32.mul_add(w2 as f32, (h2 - w2 * h) as f32);
        while px < py {
            x += 1;
            px += 2 * h2;

            if p < 0. {
                p += (h2 - px) as f32;
            } else {
                y -= 1;
                py -= 2 * w2;
                p += (h2 + px - py) as f32;
            }

            line!(x, y);
        }

        p = (h2 as f32).mul_add((x as f32 + 0.5).powi(2), (w2 * (y - 1).pow(2)) as f32)
            - (w2 * h2) as f32;
        while y > 0 {
            y -= 1;
            py -= 2 * w2;

            if p > 0. {
                p += (w2 - py) as f32;
            } else {
                x += 1;
                px += 2 * h2;
                p += (w2 + px - py) as f32;
            }

            line!(x, y);
        }
    }

    // Standard, slower brute force algorithm that iterates through all pixels
    #[allow(clippy::cast_possible_wrap)]
    fn render_circle<I: DerefMut<Target = Image<P>>>(&self, mut image: I) {
        let (h, k) = self.position;
        let (h, k) = (h as i32, k as i32);
        let r = self.radii.0 as i32;
        let r2 = r * r;

        let (mut x1, mut y1) = (h - r, k - r);
        let (mut x2, mut y2) = (h + r, k + r);

        let overlay = self.overlay.unwrap_or(image.overlay);
        let border = self
            .border
            .as_ref()
            .map(Border::bounds)
            .map(|(inner, outer, color)| {
                let inner = inner as i32;
                let outer = outer as i32;

                x1 -= outer;
                y1 -= outer;
                x2 += outer;
                y2 += outer;

                let inner = r - inner;
                let outer = r + outer;
                (inner * inner, outer * outer, color)
            });

        for y in y1..=y2 {
            for x in x1..=x2 {
                let dx = x - h;
                let dy = y - k;
                let d2 = dx * dx + dy * dy;

                if let Some((i2, o2, color)) = border {
                    if d2 >= i2 && d2 <= o2 {
                        image.overlay_pixel_with_mode(x as u32, y as u32, color, overlay);
                    }
                }

                // Inside or on the circle
                if d2 <= r2 {
                    if let Some(fill) = self.fill {
                        image.overlay_pixel_with_mode(x as u32, y as u32, fill, overlay);
                    }
                }
            }
        }
    }

    // Standard, slower brute force algorithm that iterates through all pixels
    #[allow(clippy::cast_possible_wrap, clippy::cast_precision_loss)]
    fn render_ellipse<I: DerefMut<Target = Image<P>>>(&self, mut image: I) {
        let (h, k) = self.position;
        let (h, k) = (h as i32, k as i32);
        let (a, b) = self.radii;
        let (a, b) = (a as i32, b as i32);
        let (a2, b2) = ((a * a) as f32, (b * b) as f32);

        let (mut x1, mut y1) = (h - a, k - b);
        let (mut x2, mut y2) = (h + a, k + b);

        let overlay = self.overlay.unwrap_or(image.overlay);
        let border = self
            .border
            .as_ref()
            .map(Border::bounds)
            .map(|(inner, outer, color)| {
                let inner = inner as i32;
                let outer = outer as i32;

                x1 -= outer;
                y1 -= outer;
                x2 += outer;
                y2 += outer;

                let (ia, oa) = (a - inner, a + outer - 1);
                let (ib, ob) = (b - inner, b + outer - 1);

                (
                    (ia * ia) as f32,
                    (oa * oa) as f32,
                    (ib * ib) as f32,
                    (ob * ob) as f32,
                    color,
                )
            });

        for y in y1..=y2 {
            for x in x1..=x2 {
                let dx = x - h;
                let dy = y - k;
                let dx2 = (dx * dx) as f32;
                let dy2 = (dy * dy) as f32;

                if let Some((ia2, oa2, ib2, ob2, color)) = border {
                    if dx2 / ia2 + dy2 / ib2 >= 1. && dx2 / oa2 + dy2 / ob2 <= 1. {
                        image.overlay_pixel_with_mode(x as u32, y as u32, color, overlay);
                    }
                }

                if let Some(fill) = self.fill {
                    if dx2 / a2 + dy2 / b2 <= 1. {
                        image.overlay_pixel_with_mode(x as u32, y as u32, fill, overlay);
                    }
                }
            }
        }
    }
}

impl<P: Pixel> Draw<P> for Ellipse<P> {
    fn draw<I: DerefMut<Target = Image<P>>>(&self, mut image: I) {
        assert!(
            self.fill.is_some() || self.border.is_some(),
            "must provide one of either fill or border, try calling .with_fill()"
        );
        assert!(
            self.radii.0 > 0 || self.radii.1 > 0,
            "ellipse must have non-zero radii, have you called .with_size() yet?"
        );

        if self.border.is_none() {
            if self.radii.0 == self.radii.1 {
                self.rasterize_filled_circle(image);
            } else {
                self.rasterize_filled_ellipse(image);
            }

            return;
        }

        if self.radii.0 == self.radii.1 {
            self.render_circle(image);
        } else {
            self.render_ellipse(image);
        }
    }
}

/// Pastes or overlays an image on top of another image.
#[derive(Clone)]
pub struct Paste<P: Pixel> {
    /// The position of the image to paste.
    pub position: (u32, u32),
    /// The image to paste, or the foreground image.
    pub image: Image<P>,
    /// An image that masks or filters out pixels based on the values of its own corresponding
    /// pixels.
    ///
    /// Currently this ony supports images with the [`BitPixel`] type. If you want to mask alpha
    /// values, see [`Image::mask_alpha`].
    ///
    /// If this is None, all pixels will be overlaid on top of the image.
    pub mask: Option<Image<crate::BitPixel>>,
    /// The overlay mode of the image, or None to inherit from the background image.
    pub overlay: Option<OverlayMode>,
}

impl<P: Pixel> Paste<P> {
    /// Creates a new image paste with from the given image with the position default to `(0, 0)`.
    #[must_use]
    pub const fn new(image: Image<P>) -> Self {
        Self {
            position: (0, 0),
            image,
            mask: None,
            overlay: None,
        }
    }

    /// Sets the position of where to paste the image at. The position is where the top-left corner
    /// of the image will be pasted.
    #[must_use]
    pub const fn with_position(mut self, x: u32, y: u32) -> Self {
        self.position = (x, y);
        self
    }

    /// Sets the mask image to use. Currently this is only limited to [`BitPixel`] images.
    ///
    /// This **must** have the same dimensions as the base foreground image! You will receive a
    /// panic if this is not the case.
    ///
    /// # Panics
    /// * The mask image has different dimensions than the foreground image.
    #[must_use]
    pub fn with_mask(self, mask: Image<crate::BitPixel>) -> Self {
        assert_eq!(
            self.image.dimensions(),
            mask.dimensions(),
            "mask image with dimensions {:?} has different dimensions \
            than foreground image with dimensions {:?}",
            mask.dimensions(),
            self.image.dimensions(),
        );

        // SAFETY: checked dimensions above
        unsafe { self.with_mask_unchecked(mask) }
    }

    /// Sets the mask image to use. Currently this is only limited to [`BitPixel`] images.
    ///
    /// # Safety
    /// This should have the same dimensions as the base foreground image! This method does not
    /// check for that though, however if this is not the case, you may get undescriptive panics
    /// later. Use [`Paste::with_mask`] instead if you are not 100% sure that the mask dimensions
    /// are valid.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub unsafe fn with_mask_unchecked(mut self, mask: Image<crate::BitPixel>) -> Self {
        self.mask = Some(mask);
        self
    }

    /// Sets the overlay mode of the image.
    #[must_use]
    pub const fn with_overlay_mode(mut self, mode: OverlayMode) -> Self {
        self.overlay = Some(mode);
        self
    }
}

impl<P: Pixel> Draw<P> for Paste<P> {
    fn draw<I: DerefMut<Target = Image<P>>>(&self, mut image: I) {
        let (x1, y1) = self.position;
        let (w, h) = self.image.dimensions();
        let overlay = self.overlay.unwrap_or(image.overlay);
        let mask = self.mask.as_ref();

        // These are exclusive bounds
        let (x2, y2) = (x1 + w, y1 + h);

        for (y, i) in (y1..y2).zip(0..) {
            for (x, j) in (x1..x2).zip(0..) {
                let mask = mask.map(|mask| mask.pixel(j, i).value());

                if mask.unwrap_or(true) {
                    image.overlay_pixel_with_mode(x, y, *self.image.pixel(j, i), overlay);
                }
            }
        }
    }
}
