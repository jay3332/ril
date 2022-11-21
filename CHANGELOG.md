# Changelog

All notable changes to this project will be documented in this file. This changelog was created during the development
of v0.5, therefore all changes logged prior to v0.5 may not be accurate and are only based on previous Git commits.

Versions prior to v0.5 are not tagged/released on GitHub.

## v0.6 (dev)
- Add WebP encoding and decoding support

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
