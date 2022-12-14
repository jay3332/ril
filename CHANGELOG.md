# Changelog

All notable changes to this project will be documented in this file. This changelog was created during the development
of v0.5, therefore all changes logged prior to v0.5 may not be accurate and are only based on previous Git commits.

Versions prior to v0.5 are not tagged/released on GitHub.

## v0.9 (2022-12-13)
### Breaking changes
- `Pixel::force_into_rgb[a]` method is now replaced with `Pixel::as_rgb[a]`, which also takes self by reference instead
  of by value.
- All provided `Draw` objects (but not the `Draw` trait itself) are now generic over `F: IntoFill` instead of `P: Pixel`
  - The trait `IntoFill` is explained below
  - There should be no change in usage because for any `P: Pixel`, `P` is implemented for `IntoFill`
  - If you are extracting the fill color from a `Draw` object, you will need to access the `.color()` method on
    the `SolidColor` struct. It is a `const fn`.
  - The `Draw` trait is still generic over `P: Pixel`, no changes there

### Other changes
- `ColorType::is_dynamic` is now a `const fn`
- Add `ColorType::has_alpha` for whether the color type has an alpha channel
- Add new `Fill` trait, used to represent a fill color (or gradient, see below) for a `Draw` object. This replaces the
  simple `Pixel` trait previously used for this purpose.
  - Add new `IntoFill` trait, which provides a way to convert anything to a `Fill` object
    - Associated type `<_ as IntoFill>::Pixel` is the pixel type of the fill.
    - Associated type `<_ as IntoFill>::Fill` is the actual fill type.
    - `IntoFill` is implemented for all `P: Pixel` and turns into `draw::SolidColor<P>`
    - `IntoFill` is implemented for `LinearGradient` (see below) and turns into `gradient::LinearGradientFill<P>`
- Add support for gradients
  - Enabled with the `gradient` feature, which is enabled by default
  - New `LinearGradient` struct, which represents a linear gradient
    - `LinearGradientBlendMode` and `LinearGradientInterpolation` enums are re-exports from the `colorgrad` crate,
       which is used to configure the gradient's blending mode and interpolation.
- Add `Polygon::regular` method, which creates a regular polygon with the given amount of sides, center, and radius
  - This uses the `Polygon::regular_rotated` method, which is the same method, but you are able to specify the rotation
    of the polygon in radians.

### Linear gradient example
```rust
use ril::prelude::*;

fn main() -> ril::Result<()> {
    // Create a new 256x256 RGB image with a black background
    let mut image = Image::new(256, 256, Rgb::black());
    // Create the `LinearGradient` object
    let gradient = LinearGradient::new()
        // The gradient will be rotated 45 degrees
        .with_angle_degrees(45.0)
        // The first stop is at 0.0, and is red
        .with_color(Rgb::new(255, 0, 0))
        // The second stop is at 0.5, and is white
        .with_color(Rgb::new(255, 255, 255))
        // We can also specify color stop positions manually:
        // .with_color_at(0.5, Rgb::new(255, 255, 255)) 
        // ...  
        // The third stop is at 1.0, and is green
        .with_color(Rgb::new(0, 255, 0));
    
    // Fill a hexagon with the gradient and draw it to the image
    image.draw(&Polygon::regular(6, image.center(), 64).with_fill(gradient));
    // Save the image to a PNG file
    image.save_inferred("gradient_output.png")
}
```

## v0.8 (2022-11-30)
### Breaking changes
- `Paste` draw struct now stores images and masks by reference instead of by value. This is to prevent
  unnecessary cloning of large images and/or masks.
  - Paste now has two generic lifetime arguments: `Paste<'img, 'mask, _>`.
  - This also means that `Image::paste`, `Image::paste_with_mask`, and `Image::with` methods now take images and masks
    by reference instead of by value too.

### Other changes
- Add support for drawing lines and polygons using `Line` and `Polygon` draw entities
  - Drawing a line or polygon with rounded vertices and a non-centered border position results in undesired output as
    of now.
- Add new `static` feature. When enabled, this will statically link any native dependencies
- Add non-zero width/height assertions to image constructors

#### Bug fixes
- Fix GIF decoding bug for images with a global palette
- Fix conversion using `Pixel::from_arbitrary_palette` with dynamic pixels

## v0.7 (2022-11-22)
### Breaking changes
- `ImageSequence::first_frame` now returns `Option<&Frame>` instead of `&Frame`.
  - Also introduces new `first_frame_mut` and `first_frame[_mut]_unchecked` methods.

### Other changes
- Add crate-level support for image quantization
  - The new `quantize` feature enables the `color_quant` dependency for more complex quantization algorithms  
    This is enabled by default, mainly because `color_quant` appears to not pull any additional dependencies
  - `Quantizer` struct can handle direct quantization of raw pixel data
  - `Image::quantize` is a higher-level method that can quantize an image into a paletted image
  - Implement `From<Image<Rgb[a]>>` for `Image<PalettedRgb[a]>` which utlizes quantization
- Fix decoding bug for JPEG images with L pixels

## v0.6 (2022-11-21)
- Add WebP encoding and decoding support
- Add the `all-pure` crate feature that enables all feature except for `webp`
  - This is because WASM does not support WebP since the feature uses the native `libwebp`. (as of v0.6-dev)

## v0.5 (2022-11-18)
- Rework of pixel interface
  - Remove `PixelData` enum in place of more efficient `Pixel::BIT_DEPTH` and `Pixel::COLOR_TYPE` constants
    - Includes new `Pixel::color_type` method for dynamic pixels
    - `PixelData::from_raw` is now `Pixel::from_raw_parts`
  - Implement `Hash` for pixel types
  - `Pixel::Subpixel` associated type now has bounds `Copy + Into<u8>`
  - Most places that take `&mut Image` can now take `impl DerefMut<Target = Image>` instead (e.g. draw/text methods)
  - `Pixel::from_dynamic` can now do dynamic conversions between any pixel types
- Implement all `From<Image<_>>` for `Image<T>` as an alternative to `Image::convert::<T>`
- Implement paletted images
  - `Image::from_paletted_pixels` can create an image from raw paletted pixels
  - `Image::palette[_mut][_unchecked]` methods for accessing the image's palette
  - `Image::map_palette` for mapping the image's palette
  - `Image::flatten_palette` for converting a paletted image to a non-paletted image
  - New pixel trait `Paletted<'p>` where `'p` is the lifetime of the palette (usually owned by the parent `Image`)
  - New pixel types: `PalettedRgb` and `PalettedRgba`
  - `Pixel::Color` associated type is either `Self` for non-paletted pixels or the palette's pixel type for paletted pixels
  - `Pixel::from_arbitrary_palette` for converting any raw indexed pixel to the desired pixel type
  - `Pixel::from_raw_parts_paletted` is `Pixel::from_raw_parts` that accepts a palette
- Other QoL API improvements

## v0.4 (2022-08-12)
- `Image` construct method naming changes (this is also reflected for `ImageSequence`)
  - `Image::decode_from_bytes` is now `Image::from_reader`
  - `Image::decode_inferred_from_bytes` is now `Image::from_reader_inferred`
  - Add `Image::from_bytes[_inferred]` for decoding directly from any `impl AsRef<[u8]>`
    - Useful for using with `include_bytes!` macro
    - For `ImageSequence` this will only accept slices only (`&[u8]` instead of `impl AsRef<[u8]>`)
- Add a variety of colorops
  - `Image::brighten[ed]` will brighten the image by a given amount
  - `Image::darken[ed]` will darken the image by a given amount
  - `Image::hue_rotate[d]` will rotate the hue of the image by a given amount
- Add `Pixel::Subpixel` associated type, this is the type of the subpixel (e.g. `u8` for `Rgba`)
  - Add `Pixel::map_subpixels` for mapping the subpixels of a pixel
- Add `TrueColor` pixel trait, implemented for all `Pixels` that are convertable into `Rgb` or `Rgba`

## v0.3 (2022-08-11)
- Separate various encoding dependencies and other features into cargo features
- Add text rendering support

## v0.2.1 (2022-08-09)
- Various performance improvements

## v0.2.0 (2022-08-09)
- Add GIF en/decoding support
- Add JPEG en/decoding support
- Add support for `ImageSequence` and APNGs
- Switch encoders to use external crates

## v0.1.0
Initial release
