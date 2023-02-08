//! Encloses most drawing implementations and drawable objects.

use crate::{
    fill::{Fill, IntoFill},
    Image, OverlayMode, Pixel,
};
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
    /// Creates a new border with the given color and thickness.
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

    /// Sets the thickness or width of the border.
    ///
    /// # Panics
    /// * Panics if the border thickness is 0.
    #[must_use]
    pub fn with_thickness(mut self, thickness: u32) -> Self {
        assert_ne!(thickness, 0, "border thickness cannot be 0");
        self.thickness = thickness;
        self
    }

    /// Sets the position of the border.
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

/// A line.
///
/// At its core, this method utilizes
/// [Bresenham's line algorithm](https://en.wikipedia.org/wiki/Bresenham%27s_line_algorithm),
/// and [Xiaolin Wu's line algorithm](https://en.wikipedia.org/wiki/Xiaolin_Wu%27s_line_algorithm)
/// for antialiased lines. Thicker lines are drawn as polygons.
#[derive(Clone, Debug)]
pub struct Line<F: IntoFill> {
    /// The color of the line.
    pub color: F::Fill,
    /// The overlay mode of the line, or None to inherit from the overlay mode of the image.
    pub mode: Option<OverlayMode>,
    /// The thickness of the line, in pixels. Defaults to 1.
    pub thickness: u32,
    /// The start point of the line.
    pub start: (u32, u32),
    /// The end point of the line.
    pub end: (u32, u32),
    /// Whether the line should be antialiased. Note that drawing antialiased lines is slower than
    /// drawing non-antialiased lines. Defaults to `false`.
    pub antialiased: bool,
    /// Whether the endpoints of the line should be "rounded off" with circles. Defaults to `false`.
    /// Currently, endpoints are not antialiased, regardless of the value of `antialiased`.
    ///
    /// Note that for even-numbered thicknesses, the endpoints will not be perfectly aligned with
    /// the line. For optimal results, use odd-numbered thicknesses when using enabling this field.
    pub rounded: bool,
    /// The position of the line relative to the start and end points. Defaults to `Center`
    /// (which is different from the default of `Border`).
    pub position: BorderPosition,
}

#[inline]
unsafe fn _unsafe_default_fields<F: IntoFill>() -> Line<F> {
    Line {
        color: std::mem::zeroed::<F>().into_fill(),
        mode: None,
        thickness: 1,
        start: (0, 0),
        end: (0, 0),
        antialiased: false,
        rounded: false,
        position: BorderPosition::Center,
    }
}

impl<F: IntoFill + Default> Default for Line<F> {
    fn default() -> Self {
        unsafe {
            // SAFETY: unsafe field `color` is overwritten
            _unsafe_default_fields().with_color(F::default())
        }
    }
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]
impl<F: IntoFill> Line<F> {
    /// Creates a new line.
    #[must_use]
    pub fn new(start: (u32, u32), end: (u32, u32), color: F) -> Self {
        let mut this = Self {
            color: color.into_fill(),
            start,
            end,
            // SAFETY: unsafe field `color` is overwritten
            ..unsafe { _unsafe_default_fields() }
        };
        this.update_bounding_box();
        this
    }

    fn update_bounding_box(&mut self) {
        self.color.set_bounding_box((
            self.start.0.min(self.end.0),
            self.start.1.min(self.end.1),
            self.start.0.max(self.end.0),
            self.start.1.max(self.end.1),
        ));
    }

    /// Sets the color of the line.
    #[must_use]
    pub fn with_color(mut self, color: F) -> Self {
        self.color = color.into_fill();
        self.update_bounding_box();
        self
    }

    #[allow(clippy::missing_const_for_fn)]
    pub(crate) fn with_fill_color(mut self, color: F::Fill) -> Self {
        self.color = color;
        self
    }

    /// Sets the overlay mode of the line.
    #[must_use]
    pub const fn with_mode(mut self, mode: OverlayMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Sets the thickness of the line.
    #[must_use]
    pub const fn with_thickness(mut self, thickness: u32) -> Self {
        self.thickness = thickness;
        self
    }

    /// Sets the start coordinates of the line.
    #[must_use]
    pub fn with_start(mut self, x: u32, y: u32) -> Self {
        self.start = (x, y);
        self.update_bounding_box();
        self
    }

    /// Sets the end coordinates of the line.
    #[must_use]
    pub fn with_end(mut self, x: u32, y: u32) -> Self {
        self.end = (x, y);
        self.update_bounding_box();
        self
    }

    /// Sets whether the line should be antialiased. If this is set to `true`, the overlay
    /// mode of this line will also be set to [`OverlayMode::Merge`].
    #[must_use]
    pub const fn with_antialiased(mut self, antialiased: bool) -> Self {
        self.antialiased = antialiased;
        if antialiased {
            self.mode = Some(OverlayMode::Merge);
        }
        self
    }

    /// Sets whether the line should be rounded.
    #[must_use]
    pub const fn with_rounded(mut self, rounded: bool) -> Self {
        self.rounded = rounded;
        self
    }

    /// Sets the position of the line relative to the start and end points.
    #[must_use]
    pub const fn with_position(mut self, position: BorderPosition) -> Self {
        self.position = position;
        self
    }

    fn plot_endpoints(&self, image: &mut Image<F::Pixel>) {
        if self.rounded && self.thickness > 1 {
            let (x1, y1) = self.start;
            let (x2, y2) = self.end;
            let mut reference = Ellipse::<F>::circle(x1, y1, self.thickness / 2)
                .with_fill_color(self.color.clone());

            if let Some(mode) = self.mode {
                reference = reference.with_overlay_mode(mode);
            }

            image.draw(&reference);
            image.draw(
                &reference
                    .with_position(x2, y2)
                    .with_fill_color(self.color.clone()),
            );
        }
    }

    // assumes that `x1 == x2 || y1 == y2`
    fn plot_perfect_line(&self, image: &mut Image<F::Pixel>) {
        let (mut x1, mut y1) = self.start;
        let (mut x2, mut y2) = self.end;
        let adjustment = self.thickness / 2;
        let difference = self.thickness - adjustment;

        // vertical line, adjust horizontal
        if x1 == x2 {
            x1 -= adjustment;
            x2 += difference;
        }
        // horizontal line, adjust vertical
        else {
            y1 -= adjustment;
            y2 += difference;
        }

        let mut rect =
            Rectangle::<F>::from_bounding_box(x1, y1, x2, y2).with_fill_color(self.color.clone());
        if let Some(mode) = self.mode {
            rect = rect.with_overlay_mode(mode);
        }

        image.draw(&rect);
    }

    #[inline]
    fn setup_points(&self) -> (bool, u32, u32, u32, u32) {
        let (mut x1, mut y1) = self.start;
        let (mut x2, mut y2) = self.end;

        // absolute slope is greater than 1, optimize by swapping x and y
        let swapped = y1.abs_diff(y2) > x1.abs_diff(x2);
        if swapped {
            std::mem::swap(&mut x1, &mut y1);
            std::mem::swap(&mut x2, &mut y2);
        }

        // swap start and end if necessary, this preserves the order and prevents underflow
        if x1 > x2 {
            std::mem::swap(&mut x1, &mut x2);
            std::mem::swap(&mut y1, &mut y2);
        }

        (swapped, x1, y1, x2, y2)
    }

    fn draw_thin_line(&self, image: &mut Image<F::Pixel>) {
        let (swapped, mut x1, y1, x2, y2) = self.setup_points();

        let dx = (x2 - x1) as f32;
        let dy = y1.abs_diff(y2) as f32;
        let mut err = dx / 2.0;

        let mut y = y1 as i32;
        let y_step = if y1 < y2 { 1 } else { -1 };
        let overlay = self.mode.unwrap_or(image.overlay);

        while x1 <= x2 {
            x1 += 1;
            err -= dy;
            if err < 0.0 {
                y += y_step;
                err += dx;
            }

            let (x, y) = if swapped {
                (y as u32, x1)
            } else {
                (x1, y as u32)
            };
            self.color.plot(image, x, y, overlay);
        }
    }

    fn draw_antialiased_line(&self, image: &mut Image<F::Pixel>) {
        let (swapped, x1_u, y1, x2_u, y2) = self.setup_points();
        let (x1, mut y1, x2, y2) = (x1_u as f32, y1 as f32, x2_u as f32, y2 as f32);

        let dx = x2 - x1;
        let gradient = if dx == 0.0 {
            1.0
        } else {
            // slope
            (y2 - y1) / dx
        };

        let mut x = x1_u;
        let mut lower = false;
        let overlay = self.mode.unwrap_or(image.overlay);

        while x <= x2_u {
            let fract = y1.fract();
            let mut py = y1 as u32;
            if lower {
                py += 1;
            }

            let (px, py) = if swapped { (py, x) } else { (x, py) };
            if lower {
                lower = false;
                x += 1;
                y1 += gradient;
                self.color
                    .plot_with_alpha(image, px, py, overlay, (fract * 255.0) as u8);
            } else {
                if fract > 0.0 {
                    lower = true;
                } else {
                    x += 1;
                    y1 += gradient;
                }
                self.color
                    .plot_with_alpha(image, px, py, overlay, ((1.0 - fract) * 255.0) as u8);
            }
        }
    }

    fn draw_thick_line(&self, image: &mut Image<F::Pixel>) {
        let (x1, y1) = self.start;
        let (x2, y2) = self.end;
        let (x1, y1, x2, y2) = (x1 as f32, y1 as f32, x2 as f32, y2 as f32);

        let mut angle = (y2 - y1).atan2(x2 - x1);
        let polygon = if self.position == BorderPosition::Center {
            let upper = angle + std::f32::consts::FRAC_PI_2;
            let lower = angle - std::f32::consts::FRAC_PI_2;

            let thickness = self.thickness as f32 / 2.0;
            let upper_cos = thickness * upper.cos();
            let upper_sin = thickness * upper.sin();
            let lower_cos = thickness * lower.cos();
            let lower_sin = thickness * lower.sin();

            Polygon::<F>::from_vertices([
                ((x1 + upper_cos) as u32, (y1 + upper_sin) as u32),
                ((x1 + lower_cos) as u32, (y1 + lower_sin) as u32),
                ((x2 + lower_cos) as u32, (y2 + lower_sin) as u32),
                ((x2 + upper_cos) as u32, (y2 + upper_sin) as u32),
            ])
        } else {
            if self.position == BorderPosition::Inset {
                angle += std::f32::consts::PI;
            } else {
                angle -= std::f32::consts::PI;
            }

            let thickness = self.thickness as f32;
            let cos = thickness * angle.cos();
            let sin = thickness * angle.sin();

            Polygon::<F>::from_vertices([
                ((x1 + cos) as u32, (y1 + sin) as u32),
                ((x1 - cos) as u32, (y1 - sin) as u32),
                ((x2 - cos) as u32, (y2 - sin) as u32),
                ((x2 + cos) as u32, (y2 + sin) as u32),
            ])
        };

        image.draw(
            &polygon
                .with_fill_color(self.color.clone())
                .with_antialiased(self.antialiased),
        );
    }
}

impl<F: IntoFill> Draw<F::Pixel> for Line<F> {
    fn draw<I: DerefMut<Target = Image<F::Pixel>>>(&self, mut image: I) {
        let (x1, y1) = self.start;
        let (x2, y2) = self.end;
        let image = &mut *image;

        // TODO: still have to adjust endpoints for lines with adjusted positions
        // TODO: make endpoints smoothly adjust to gradient fills
        self.plot_endpoints(image);

        if x1 == x2 || y1 == y2 {
            self.plot_perfect_line(image);
        } else if self.thickness == 1 {
            if self.antialiased {
                self.draw_antialiased_line(image);
            } else {
                self.draw_thin_line(image);
            }
        } else {
            self.draw_thick_line(image);
        }
    }
}

/// A polygon.
#[derive(Clone, Debug)]
pub struct Polygon<F: IntoFill> {
    /// A `Vec` of vertices that make up the polygon. The vertices are connected in the order they
    /// are given.
    ///
    /// When drawing, a panic will occur if there is less than 3 vertices in this `Vec`.
    ///
    /// If the first and last vertices are the same, these points will remain untouched. Otherwise,
    /// and extra vertex equivalent to the first vertex will be added to the end of the `Vec` to
    /// close the polygon.
    pub vertices: Vec<(u32, u32)>,
    /// The border of the polygon. Either this or `fill` must be `Some`.
    pub border: Option<Border<F::Pixel>>,
    /// Whether the border should be rounded off by drawing circles at each vertex. This is only
    /// applied if `border` is `Some`. Additionally, these circles will not antialias.
    pub rounded: bool,
    /// The fill color of the polygon. Either this or `border` must be `Some`.
    pub fill: Option<F::Fill>,
    /// The overlay mode of the polygon. If `None`, the image's overlay mode will be used.
    pub overlay: Option<OverlayMode>,
    /// Whether to antialias the polygon's edges.
    pub antialiased: bool,
}

impl<F: IntoFill> Default for Polygon<F> {
    fn default() -> Self {
        Self {
            vertices: Vec::default(),
            border: None,
            rounded: false,
            fill: None,
            overlay: None,
            antialiased: false,
        }
    }
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]
impl<F: IntoFill> Polygon<F> {
    /// Creates a new empty polygon.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates the polygon's bounding box. This is automatically called, unless explicitly
    /// specified in documentation.
    pub fn update_bounding_box(&mut self) {
        if let Some(ref mut fill) = self.fill {
            let x_iter = self.vertices.iter().map(|(x, _)| *x);
            let y_iter = self.vertices.iter().map(|(_, y)| *y);

            fill.set_bounding_box((
                x_iter.clone().min().unwrap_or(0),
                y_iter.clone().min().unwrap_or(0),
                x_iter.max().unwrap_or(0),
                y_iter.max().unwrap_or(0),
            ));
        }
    }

    /// Creates a new polygon with the given vertices.
    #[must_use]
    pub fn from_vertices(vertices: impl IntoIterator<Item = (u32, u32)>) -> Self {
        Self {
            vertices: vertices.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Creates a regular polygon with `n` sides centered at `center`, with each of the points
    /// `radius` away from the center. `angle` is measured in radians and is the angle offset of the
    /// first point, which for `angle=0.0`, will be horizontally to the right of the center
    /// (i.e. a unit circle).
    ///
    /// For angles specified in degrees, the [`f64::to_radians`] method can be used for conversion.
    ///
    /// # Note
    /// Apart from `n=4`, the polygon will not be pixel-perfect since no polygons other than squares
    /// have all of their vertices as perfect integers. This means that they will be rounded to the
    /// nearest pixel.
    ///
    /// # Panics
    /// * If `n < 3`
    #[must_use]
    #[allow(clippy::cast_lossless)]
    pub fn regular_rotated(n: u32, center: (u32, u32), radius: u32, angle: f64) -> Self {
        assert!(n >= 3, "n must be greater than or equal to 3");

        let mut vertices = Vec::with_capacity(n as usize);
        let base = std::f64::consts::TAU / n as f64;
        let (cx, cy) = (center.0 as f64, center.1 as f64);
        let radius = radius as f64;

        for i in 0..n {
            let (angle_sin, angle_cos) = base.mul_add(i as f64, -angle).sin_cos();
            let x = radius.mul_add(angle_cos, cx).round() as u32;
            let y = radius.mul_add(angle_sin, cy).round() as u32;

            vertices.push((x, y));
        }

        Self::from_vertices(vertices)
    }

    /// Creates a regular polygon with the first point vertically up from the center (the polygon
    /// will seem to be facing "upwards").
    ///
    /// This is a shortcut to calling [`Polygon::regular_rotated`] with `angle = PI / 2` (`90deg`).
    ///
    /// # See Also
    /// * [`Polygon::regular_rotated`] for more information.
    #[must_use]
    pub fn regular(n: u32, center: (u32, u32), radius: u32) -> Self {
        Self::regular_rotated(n, center, radius, std::f64::consts::FRAC_PI_2)
    }

    /// Adds a vertex to the polygon.
    #[must_use]
    pub fn with_vertex(mut self, x: u32, y: u32) -> Self {
        self.push_vertex(x, y);
        self.update_bounding_box();
        self
    }

    /// Adds a vertex to the polygon in place.
    pub fn push_vertex(&mut self, x: u32, y: u32) {
        self.vertices.push((x, y));
    }

    /// Returns a slice of the vertices in the polygon.
    #[must_use]
    pub fn vertices(&self) -> &[(u32, u32)] {
        &self.vertices
    }

    /// Returns a mutable slice of the vertices in the polygon.
    #[must_use]
    pub fn vertices_mut(&mut self) -> &mut [(u32, u32)] {
        &mut self.vertices
    }

    /// Iterates over the vertices in the polygon.
    pub fn iter_vertices(&self) -> impl Iterator<Item = &(u32, u32)> {
        self.vertices.iter()
    }

    /// Iterates over the vertices in the polygon in mutable form. Make sure to call
    /// [`update_bounding_box`](#method.update_bounding_box) after mutating the vertices.
    pub fn iter_vertices_mut(&mut self) -> impl Iterator<Item = &mut (u32, u32)> {
        self.vertices.iter_mut()
    }

    /// Sets the border of the polygon.
    #[must_use]
    pub const fn with_border(mut self, border: Border<F::Pixel>) -> Self {
        self.border = Some(border);
        self
    }

    /// Sets whether the border should be rounded.
    #[must_use]
    pub const fn with_rounded(mut self, rounded: bool) -> Self {
        self.rounded = rounded;
        self
    }

    /// Sets the fill color of the polygon.
    #[must_use]
    pub fn with_fill(mut self, fill: F) -> Self {
        self.fill = Some(fill.into_fill());
        self.update_bounding_box();
        self
    }

    #[allow(clippy::missing_const_for_fn)]
    pub(crate) fn with_fill_color(mut self, fill: F::Fill) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Sets the overlay mode of the polygon.
    #[must_use]
    pub const fn with_overlay_mode(mut self, overlay: OverlayMode) -> Self {
        self.overlay = Some(overlay);
        self
    }

    /// Sets whether to antialias the polygon's edges. If set to `true`, this will also set the
    /// overlay mode to [`OverlayMode::Merge`].
    #[must_use]
    pub const fn with_antialiased(mut self, antialiased: bool) -> Self {
        self.antialiased = antialiased;
        if antialiased {
            self.overlay = Some(OverlayMode::Merge);
        }
        self
    }

    #[inline]
    fn sanitize_vertices(&self) -> Vec<(u32, u32)> {
        assert!(
            self.vertices.len() >= 3,
            "polygon must have at least 3 vertices"
        );

        let mut vertices = self.vertices.clone();
        if vertices.first() != vertices.last() {
            // SAFETY: assertion above ensures that there are at least 3 points
            vertices.push(unsafe { *self.vertices.get_unchecked(0) });
        }
        vertices
    }

    fn rasterize_fill(&self, image: &mut Image<F::Pixel>, vertices: &[(u32, u32)]) {
        let vertices = vertices
            .iter()
            .map(|(x, y)| (*x as i32, *y as i32))
            .collect::<Vec<_>>();

        // SAFETY: assertion in `sanitize_vertices` ensures that there are at least 3 points
        let (y_min, y_max) = unsafe {
            macro_rules! y_iter {
                ($meth:ident) => {{
                    vertices
                        .iter()
                        .map(|(_, y)| *y)
                        .$meth()
                        .unwrap_unchecked()
                        .min(image.height() as i32 - 1)
                }};
            }

            (y_iter!(min), y_iter!(max))
        };
        // SAFETY: this method is only called if `self.fill` is `Some`
        let fill = unsafe { self.fill.as_ref().unwrap_unchecked() };
        let overlay = self.overlay.unwrap_or(image.overlay);
        let mut intersections = Vec::new();

        (y_min..=y_max).for_each(|y| {
            vertices.windows(2).for_each(|edge| unsafe {
                let &(x1, y1) = edge.get_unchecked(0);
                let &(x2, y2) = edge.get_unchecked(1);

                if y1 <= y && y2 >= y || y2 <= y && y1 >= y {
                    if y1 == y2 {
                        intersections.push(x1);
                        intersections.push(x2);
                    } else if y1 == y || y2 == y {
                        if y2 > y {
                            intersections.push(x1);
                        }
                        if y1 > y {
                            intersections.push(x2);
                        }
                    } else {
                        let frac = (y - y1) as f32 / (y2 - y1) as f32;
                        intersections.push(frac.mul_add((x2 - x1) as f32, x1 as f32) as _);
                    }
                }
            });

            intersections.sort_unstable();
            intersections.chunks_exact(2).for_each(|range| {
                for x in range[0]..=range[1] {
                    fill.plot(image, x as u32, y as u32, overlay);
                }
            });
            intersections.clear();
        });
    }
}

impl<F: IntoFill> Draw<F::Pixel> for Polygon<F> {
    fn draw<I: DerefMut<Target = Image<F::Pixel>>>(&self, mut image: I) {
        debug_assert!(
            self.fill.is_some() || self.border.is_some(),
            "polygon must have a fill or border"
        );

        let image = &mut *image;
        let vertices = self.sanitize_vertices();

        if let Some(ref fill) = self.fill {
            self.rasterize_fill(image, &vertices);

            if self.border.is_none() && self.antialiased {
                for edge in vertices.windows(2) {
                    unsafe {
                        // SAFETY: windows(2) ensures that there are at least 2 points
                        let &from = edge.get_unchecked(0);
                        let &to = edge.get_unchecked(1);
                        image.draw(
                            // SAFETY: color is overridden
                            &Line::new(from, to, std::mem::zeroed::<F>())
                                .with_fill_color(fill.clone())
                                .with_antialiased(true),
                        );
                    }
                }
            }
        }

        if let Some(ref border) = self.border {
            for edge in vertices.windows(2) {
                unsafe {
                    // SAFETY: windows(2) ensures that there are at least 2 points
                    let &from @ (x, y) = edge.get_unchecked(0);
                    let &to = edge.get_unchecked(1);
                    image.draw(
                        &Line::new(from, to, border.color)
                            .with_antialiased(self.antialiased)
                            .with_thickness(border.thickness)
                            .with_position(border.position),
                    );

                    if self.rounded {
                        image.draw(
                            &Ellipse::circle(x, y, border.thickness / 2).with_fill(border.color),
                        );
                    }
                }
            }
        }
    }
}

/// A rectangle.
///
/// # Note
/// You must specify a width and height for the rectangle with something such as [`with_size`].
/// If you don't, a panic will be raised during drawing. You can also try using
/// [`from_bounding_box`] to create a rectangle from a bounding box, which automatically fills
/// in the size.
///
/// Additionally, a panic will be raised during drawing if you do not specify either a fill color
/// or a border. these can be set with [`with_fill`] and [`with_border`] respectively.
#[derive(Clone, Debug)]
pub struct Rectangle<F: IntoFill> {
    /// The position of the rectangle. The top-left corner of the rectangle will be rendered at
    /// this position.
    pub position: (u32, u32),
    /// The dimensions of the rectangle, in pixels.
    pub size: (u32, u32),
    /// The border data of the rectangle, or None if there is no border.
    pub border: Option<Border<F::Pixel>>,
    /// The fill color of the rectangle, or None if there is no fill.
    pub fill: Option<F::Fill>,
    /// The overlay mode of the rectangle, or None to inherit from the overlay mode of the image.
    pub overlay: Option<OverlayMode>,
}

impl<F: IntoFill> Default for Rectangle<F> {
    fn default() -> Self {
        Self {
            position: (0, 0),
            size: (0, 0),
            border: None,
            fill: None,
            overlay: None,
        }
    }
}

impl<F: IntoFill> Rectangle<F> {
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
    #[deprecated = "use `Rectangle::at` instead"]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new rectangle at the specified coordinates with default values.
    ///
    /// # Note
    /// You must explicitly set the size of the rectangle with [`with_size`]. If no size is set
    /// before drawing, you will receive a panic.
    #[must_use]
    pub fn at(x: u32, y: u32) -> Self {
        Self::default().with_position(x, y)
    }

    fn update_bounding_box(&mut self) {
        if let Some(ref mut fill) = self.fill {
            let (x, y) = self.position;
            let (w, h) = self.size;
            fill.set_bounding_box((x, y, x + w, y + h));
        }
    }

    /// Creates a new square with side length `s` with the top-left corner at the given coordinates.
    #[must_use]
    pub fn square(s: u32, (x, y): (u32, u32)) -> Self {
        Self::at(x, y).with_size(s, s)
    }

    /// Creates a new rectangle from two coordinates specified as 4 parameters.
    ///
    /// The first coordinate is the top-left corner of the rectangle, and the second coordinate is
    /// the bottom-right corner of the rectangle.
    #[must_use]
    pub fn from_bounding_box(mut x1: u32, mut y1: u32, mut x2: u32, mut y2: u32) -> Self {
        if x1 > x2 {
            std::mem::swap(&mut x1, &mut x2);
        }
        if y1 > y2 {
            std::mem::swap(&mut y1, &mut y2);
        }

        Self::default()
            .with_position(x1, y1)
            .with_size(x2 - x1, y2 - y1)
    }

    /// Sets the position of the rectangle.
    #[must_use]
    pub fn with_position(mut self, x: u32, y: u32) -> Self {
        self.position = (x, y);
        self.update_bounding_box();
        self
    }

    /// Sets the size of the rectangle in pixels.
    #[must_use]
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self.update_bounding_box();
        self
    }

    /// Sets the border information of the rectangle.
    ///
    /// # See Also
    /// * [`Border`]
    #[must_use]
    pub const fn with_border(mut self, border: Border<F::Pixel>) -> Self {
        self.border = Some(border);
        self
    }

    /// Sets the fill color of the rectangle.
    #[must_use]
    pub fn with_fill(mut self, fill: F) -> Self {
        self.fill = Some(fill.into_fill());
        self.update_bounding_box();
        self
    }

    #[allow(clippy::missing_const_for_fn)]
    pub(crate) fn with_fill_color(mut self, fill: F::Fill) -> Self {
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

impl<F: IntoFill> Draw<F::Pixel> for Rectangle<F> {
    fn draw<I: DerefMut<Target = Image<F::Pixel>>>(&self, mut image: I) {
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
        if let Some(ref fill) = self.fill {
            for y in y1..y2 {
                for x in x1..x2 {
                    fill.plot(&mut image, x, y, overlay);
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
#[derive(Clone, Debug)]
pub struct Ellipse<F: IntoFill> {
    /// The center position of the ellipse.
    /// The center of this ellipse will be rendered at this position.
    pub position: (u32, u32),
    /// The radii of the ellipse, in pixels; (horizontal, vertical).
    pub radii: (u32, u32),
    // The border data for the ellipse if any.
    pub border: Option<Border<F::Pixel>>,
    // The fill color for the ellipse if any.
    pub fill: Option<F::Fill>,
    // The overlay mode for the ellipse or None to inherit from the image's overlay mode.
    pub overlay: Option<OverlayMode>,
}

impl<F: IntoFill> Default for Ellipse<F> {
    fn default() -> Self {
        Self {
            position: (0, 0),
            radii: (0, 0),
            border: None,
            fill: None,
            overlay: None,
        }
    }
}

impl<F: IntoFill> Ellipse<F> {
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

    fn update_bounding_box(&mut self) {
        if let Some(ref mut fill) = self.fill {
            fill.set_bounding_box((
                self.position.0 - self.radii.0,
                self.position.1 - self.radii.1,
                self.position.0 + self.radii.0,
                self.position.1 + self.radii.1,
            ));
        }
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
    pub fn with_position(mut self, x: u32, y: u32) -> Self {
        self.position = (x, y);
        self.update_bounding_box();
        self
    }

    /// Sets the radii of the ellipse in pixels.
    #[must_use]
    pub fn with_radii(mut self, width: u32, height: u32) -> Self {
        self.radii = (width, height);
        self.update_bounding_box();
        self
    }

    /// Sets the diameters of the ellipse in pixels.
    #[must_use]
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.radii = (width / 2, height / 2);
        self.update_bounding_box();
        self
    }

    /// Sets the border of the ellipse.
    #[must_use]
    pub const fn with_border(mut self, border: Border<F::Pixel>) -> Self {
        self.border = Some(border);
        self
    }

    /// Sets the fill color of the ellipse.
    #[must_use]
    pub fn with_fill(mut self, fill: F) -> Self {
        self.fill = Some(fill.into_fill());
        self.update_bounding_box();
        self
    }

    #[allow(clippy::missing_const_for_fn)]
    pub(crate) fn with_fill_color(mut self, fill: F::Fill) -> Self {
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
    fn rasterize_filled_circle(&self, image: &mut Image<F::Pixel>) {
        let radius = self.radii.0 as i32;

        let mut x = 0;
        let mut y = radius;
        let mut p = 1 - radius;

        let (h, k) = self.position;
        let (h, k) = (h as i32, k as i32);

        #[allow(unused_variables)] // rust knows this, but the external linter doesn't
        let fill = self.fill.as_ref().unwrap();
        #[allow(unused_variables)] // rust knows this, but the external linter doesn't
        let overlay = self.overlay.unwrap_or(image.overlay);

        macro_rules! line {
            ($from:expr, $to:expr, $y:expr) => {{
                let y = $y as u32;

                for x in ($from as u32)..=($to as u32) {
                    fill.plot(image, x, y, overlay);
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
    fn rasterize_filled_ellipse(&self, image: &mut Image<F::Pixel>) {
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
        let fill = self.fill.as_ref().unwrap();
        #[allow(unused_variables)] // rust knows this, but the external linter doesn't
        let overlay = self.overlay.unwrap_or(image.overlay);

        macro_rules! line {
            ($from:expr, $to:expr, $y:expr) => {{
                let y = ($y) as u32;

                for x in ($from)..=($to) {
                    fill.plot(image, x as u32, y, overlay);
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
    fn render_circle(&self, image: &mut Image<F::Pixel>) {
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
                    if let Some(ref fill) = self.fill {
                        fill.plot(image, x as u32, y as u32, overlay);
                    }
                }
            }
        }
    }

    // Standard, slower brute force algorithm that iterates through all pixels
    #[allow(clippy::cast_possible_wrap, clippy::cast_precision_loss)]
    fn render_ellipse(&self, image: &mut Image<F::Pixel>) {
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

                if let Some(ref fill) = self.fill {
                    if dx2 / a2 + dy2 / b2 <= 1.0 {
                        fill.plot(image, x as u32, y as u32, overlay);
                    }
                }
            }
        }
    }
}

impl<F: IntoFill> Draw<F::Pixel> for Ellipse<F> {
    fn draw<I: DerefMut<Target = Image<F::Pixel>>>(&self, mut image: I) {
        assert!(
            self.fill.is_some() || self.border.is_some(),
            "must provide one of either fill or border, try calling .with_fill()"
        );
        assert!(
            self.radii.0 > 0 || self.radii.1 > 0,
            "ellipse must have non-zero radii, have you called .with_size() yet?"
        );

        let image = &mut *image;

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
pub struct Paste<'img, 'mask, P: Pixel> {
    /// The position of the image to paste.
    pub position: (u32, u32),
    /// A reference to the image to paste, or the foreground image.
    pub image: &'img Image<P>,
    /// A refrence to an image that masks or filters out pixels based on the values of its own
    /// corresponding pixels.
    ///
    /// Currently this ony supports images with the [`BitPixel`] type. If you want to mask alpha
    /// values, see [`Image::mask_alpha`].
    ///
    /// If this is None, all pixels will be overlaid on top of the image.
    pub mask: Option<&'mask Image<crate::BitPixel>>,
    /// The overlay mode of the image, or None to inherit from the background image.
    pub overlay: Option<OverlayMode>,
}

impl<'img, 'mask, P: Pixel> Paste<'img, 'mask, P> {
    /// Creates a new image paste with from the given image with the position default to `(0, 0)`.
    #[must_use]
    pub const fn new(image: &'img Image<P>) -> Self {
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
    pub fn with_mask(self, mask: &'mask Image<crate::BitPixel>) -> Self {
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
    pub unsafe fn with_mask_unchecked(mut self, mask: &'mask Image<crate::BitPixel>) -> Self {
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

impl<'img, 'mask, P: Pixel> Draw<P> for Paste<'img, 'mask, P> {
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
