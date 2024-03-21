<h1 align="center" id="ril">ril</h1>
<p align="center">
  <sup>
    <b>R</b>ust <b>I</b>maging <b>L</b>ibrary: A performant and high-level Rust imaging crate.
    <br>
    <a href="https://docs.rs/ril"><b>Documentation</b></a> •
    <a href="https://crates.io/crates/ril"><b>Crates.io</b></a> •
    <a href="https://discord.gg/FqtZ6akWpd"><b>Discord</b></a>
  </sup>
</p>

## ril v0.10 has been published to crates.io!
This is the biggest release since the beginning of ril and includes a lot of breaking changes.

### Notable Additions:
- Major refactor of `Encoder` trait
- Support for lazy image sequence encoding
- Tile resize
- Radial and conic gradients

### [See the changelog](https://github.com/jay3332/ril/blob/main/CHANGELOG.md) for more information.

## What's this?
RIL (Rust Imaging Library) is a Rust crate designed to provide an easy-to-use, high-level interface
around image processing in Rust. Image and animation processing has never been
this easy before, and it's hard to find a good crate for it.

RIL was designed not only for static single-frame images in mind, but also for
animated images such as GIFs or APNGs that have multiple frames. RIL provides a
streamlined API for this.

Even better, benchmarks prove that RIL, even with its high-level interface, is as
performant and usually even faster than leading imaging crates such as `image-rs`. See
[benchmarks](#benchmarks) for more information.

## Features
- Support for encoding from/decoding to a wide range of image formats
- Variety of image processing and manipulation operations, including drawing
- Robust support for animated images such as GIFs via FrameIterator and ImageSequence
  - See [Animated Image Support](#animated-image-support) for more information.
- Robust and performant support for fonts and text rendering
  - See [Rendering Text](#rendering-text) for more information.
- A streamlined front-facing interface

## Support
⚠ This crate is a work in progress

By the first stable release, we plan to support the following image encodings:

| Encoding Format | Current Status    |
|-----------------|-------------------|
| PNG/APNG        | Supported         |
| JPEG            | Supported         |
| GIF             | Supported         |
| WebP            | Supported         |
| BMP             | Not yet supported |
| TIFF            | Not yet supported |

Additionally, we also plan to support the following pixel formats:

| Pixel Format                           | Current Status              |
|----------------------------------------|-----------------------------|
| RGB8                                   | Supported as `Rgb`          |
| RGBA8                                  | Supported as `Rgba`         |
| L8 (grayscale)                         | Supported as `L`            |
| LA8 (grayscale + alpha)                | Not yet supported           |
| 1 (single-bit pixel, equivalent to L1) | Supported as `BitPixel`     |
| Indexed RGB8 (palette)                 | Supported as `PalettedRgb`  |
| Indexed RGBA8 (palette)                | Supported as `PalettedRgba` |

16-bit pixel formats are currently downscaled to 8-bits. We do plan to
have actual support 16-bit pixel formats in the future.

## Requirements
MSRV (Minimum Supported Rust Version) is v1.72.0.

## Installation
Add the following to your `Cargo.toml` dependencies:
```toml
ril = { version = "0", features = ["all"] }
```

Or, you can run `cargo add ril --features=all` if you have Rust 1.62.0 or newer.

The above enables all features. See [Cargo Features](#cargo-features) for more information on how you can
tune these features to reduce dependencies.

### Linking errors on Windows
If you get errors regarding `link.exe` on Windows, this is because `libwebp` has problems linking with Windows for now.

This will be resolved when WebP encoders/decoders are rewritten in pure Rust. For now, you can use switch out the
`all` feature with `all-pure`:

```toml
ril = { version = "0", features = ["all-pure"] }
```

This will hopefully resolve the linking errors, at the cost of not having WebP support.

## Benchmarks

### Decode GIF + Invert each frame + Encode GIF (600x600, 77 frames)
Performed locally (10-cores) ([Source](https://github.com/jay3332/ril/blob/main/benches/invert_comparison.rs))

| Benchmark                                     | Time (average of runs in 10 seconds, lower is better) |
|-----------------------------------------------|-------------------------------------------------------|
| ril (combinator)                              | 902.54 ms                                             |
| ril (for-loop)                                | 922.08 ms                                             |
| ril (low-level hardcoded GIF en/decoder)      | 902.28 ms                                             |
| image-rs (low-level hardcoded GIF en/decoder) | 940.42 ms                                             |
| Python, wand (ImageMagick)                    | 1049.09 ms                                            |

### Rasterize and render text (Inter font, 20px, 1715 glyphs)
Performed locally (10-cores) ([Source](https://github.com/jay3332/ril/blob/main/benches/text_comparison.rs))

| Benchmark                                     | Time (average of runs in 10 seconds, lower is better) |
|-----------------------------------------------|-------------------------------------------------------|
| ril (combinator)                              | 1.5317 ms                                             |
| image-rs + imageproc                          | 2.4332 ms                                             |

## Cargo Features
RIL currently depends on a few dependencies for certain features - especially for various image encodings.
By default RIL comes with no encoding dependencies but with the `text` and `resize` dependencies, which give you text
and resizing capabilities respectively.

You can use the `all` feature to enable all features, including encoding features. This enables the widest range of
image format support, but adds a lot of dependencies you may not need.

For every image encoding that requires a dependency, a corresponding feature can be enabled for it:

| Encoding     | Feature               | Dependencies                   | Default? |
|--------------|-----------------------|--------------------------------|----------|
| PNG and APNG | `png`                 | `png`                          | no       |
| JPEG         | `jpeg`                | `jpeg-decoder`, `jpeg-encoder` | no       |
| GIF          | `gif`                 | `gif`                          | no       |
| WebP         | `webp` or `webp-pure` | `libwebp-sys2` or `image-webp` | no       |

Other features:

| Description                                                                            | Feature      | Dependencies        | Default? |
|----------------------------------------------------------------------------------------|--------------|---------------------|----------|
| Font/Text Rendering                                                                    | `text`       | `fontdue`           | yes      |
| Image Resizing                                                                         | `resize`     | `fast_image_resize` | yes      |
| Color Quantization (using NeuQuant)                                                    | `quantize`   | `color_quant`       | yes      |
| Gradients                                                                              | `gradient`   | `colorgrad`         | yes      |
| Enable all features,<br/> including all encoding features (excludes `nightly` feature) | `all`        |                     | no       |

### WebP Support limitations
WebP support uses `libwebp`, which is a native library. This means that if you try to use the `webp` feature
when compiling to a WebAssembly target, it might fail. We plan on making a pure-Rust port of `libwebp` in the future.

For ease of use, the `all-pure` feature is provided, which is the equivalent of `all` minus the `webp` feature.

## Examples

#### Open an image, invert it, and then save it:
```rust
use ril::prelude::*;

fn main() -> ril::Result<()> {
    let image = !Image::open("sample.png")?; // notice the `!` operator
    image.save_inferred("inverted.png")?;
    
    Ok(())
}
```

or, why not use method chaining?
```rust
Image::open("sample.png")?
    .not()  // std::ops::Not trait
    .save_inferred("inverted.png")?;
```

#### Create a new black image, open the sample image, and paste it on top of the black image:
```rust
let image = Image::new(600, 600, Rgb::black());
image.paste(100, 100, Image::open("sample.png")?);
image.save_inferred("sample_on_black.png")?;
```

you can still use method chaining, but this accesses a lower level interface:
```rust
let image = Image::new(600, 600, Rgb::black())
    .with(&Paste::new(Image::open("sample.png")?).with_position(100, 100))
    .save_inferred("sample_on_black.png")?;
```

#### Open an image and mask it to a circle:
```rust
let image = Image::<Rgba>::open("sample.png")?;
let (width, height) = image.dimensions();

let ellipse = 
    Ellipse::from_bounding_box(0, 0, width, height).with_fill(L(255));

let mask = Image::new(width, height, L(0));
mask.draw(&ellipse);

image.mask_alpha(&mask);
image.save_inferred("sample_circle.png")?;
```

### Animated Image Support
RIL supports high-level encoding, decoding, and processing of animated images of any format,
such as GIF or APNGs.

Animated images can be lazily decoded. This means you can process the frames of an animated image
one by one as each frame is decoded. This can lead to huge performance and memory gains when compared to 
decoding all frames at once, processing those frames individually, and then encoding the image back to a file.

For lazy animated image decoding, the `FrameIterator` trait is used as a high-level iterator interface
to iterate through all frames of an animated image, lazily. These implement `Iterator<Item = Frame<_>>`.

For times when you need to collect all frames of an image, `ImageSequence` is used as a high-level
interface around a sequence of images. This can hold extra metadata about the animation such as loop count.

#### Open an animated image and invert each frame as they are decoded, then saving them:

```rust
let mut output = ImageSequence::<Rgba>::new();

// ImageSequence::open is lazy
for frame in ImageSequence::<Rgba>::open("sample.gif")? {
    let frame = frame?;
    frame.invert();
    output.push(frame);

    // or...
    output.push_frame(!frame?);
}

output.save_inferred("inverted.gif")?;
```

#### Or, how about we encode each frame immediately without storing them in memory?
```rust
let mut stream = ImageSequence::<Rgba>::open("sample.gif")?;

// Use the first frame to initialize the encoder
let mut output = File::create("inverted.gif")?;
let first_frame = stream.next().unwrap()?;
let mut encoder = GifEncoder::new(&mut output, &first_frame)?;

// Then, write the first frame into the GIF
encoder.add_frame(&first_frame)?;

// Now, invert each frame and write it into the GIF
for frame in stream {
    encoder.add_frame(&!frame?)?;
}
```

#### Open an animated image and save each frame into a separate PNG image as they are decoded:
```rust
ImageSequence::<Rgba>::open("sample.gif")?
    .enumerate()
    .for_each(|(idx, frame)| {
        frame
            .unwrap()
            .save_inferred(format!("frames/{}.png", idx))
            .unwrap();
    });
```

Additionally, `Frame`s house `Image`s, but they are not `Image`s themselves. However, `Frame`s are able
to dereference into `Image`s, so calling image methods on frames will seem transparent.

### Rendering Text
RIL provides a streamlined interface for rendering text.

There are two ways to render text: with a `TextSegment` or with a `TextLayout`. A `TextSegment`
is faster and more lightweight than a `TextLayout` (and it's cloneable, unlike `TextLayout`), but
lacks many of the features of a `TextLayout`.

A `TextSegment` supports only one font and either represents a segment in a `TextLayout`, or it can
be directly rendered more efficiently than a `TextLayout`. You should only use `TextLayout` if you 
need what `TextSegment` can't provide.

`TextLayout`s support anchor-style text-alignment, and can be used to render text with multiple fonts
and styles, such as different sizes or colors. It also provides the ability to grab the dimensions
of the text before rendering such as width and height. `TextSegment` cannot do this.

#### Render text with a `TextSegment`:
```rust
let mut image = Image::new(512, 256, Rgb::black());
// Open the font at the given path. You can try using `Font::from_bytes` along with the `include_bytes!` macro
// since fonts can usually be statically loaded.
let font = Font::open(
    "Arial.ttf",
    // Do note that the following is a specified optimal size
    // and not a fixed size for the font. It specifies what size
    // to optimize rasterizing for. You do not have to load the same
    // font multiple times for different sizes.
    36.0,
)?;

let text = TextSegment::new(&font, "Hello, world", Rgb::white())
    .with_position(20, 20);

image.draw(&text);
image.save_inferred("text.png")?;
```

#### Render text in the center of the image with a `TextLayout`:
```rust
let mut image = Image::new(512, 256, Rgb::black());
let font = Font::open("Arial.ttf", 36.0)?;
let bold = Font::open("Arial Bold.ttf", 36.0)?;

let (x, y) = image.center();
let layout = TextLayout::new()
    .centered() // Shorthand for centering horizontally and vertically
    .with_wrap(WrapStyle::Word) // RIL supports word wrapping
    .with_width(image.width()) // This is the width to wrap text at. Only required if you want to wrap text.
    .with_position(x, y); // Position the anchor (which is the center) at the center of the image
    .with_segment(&TextSegment::new(&font, "Here is some ", Rgb::white()))
    .with_segment(&TextSegment::new(&bold, "bold ", Rgb::white()))
    .with_segment(&TextSegment::new(&font, "text.", Rgb::white()));

image.draw(&layout);
```

## Contributing
See [CONTRIBUTING.md](https://github.com/jay3332/ril/blob/main/CONTRIBUTING.md) for more information.
