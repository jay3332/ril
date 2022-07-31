# ril
**R**ust **I**maging **L**ibrary: A high-level Rust imaging crate.

## What's this?
This is a Rust crate designed to provide an easy-to-use, high-level interface
around image processing in Rust. Image and animation processing has never been
this easy before, and it's hard to find a good crate for it.

## Support
⚠ This crate is a work in progress

By the first stable release, we plan to support the following image encodings:

| Encoding Format | Current Status     |
|-----------------|--------------------|
| PNG\* (encoder) | Work in Progress   |
| PNG\* (decoder) | Work in Progress   |
| JPEG            | Not yet supported  |
| GIF             | Not yet supported  |
| WebP            | Not yet supported  |
| BMP             | Not yet supported  |
| TIFF            | Not yet supported  |

\* PNG encoding *does* account for APNG. (APNG is not yet supported)

## Installation
Use GitHub until stable release.

TODO: proper installation instructions

## Examples

#### Open an image, invert it, and then save it:
```rs
use ril::prelude::*;

fn main() -> ril::Result<()> {
    let image = Image::open("sample.png")?;
    image.invert();
    image.save_inferred("inverted.png")?;
    
    Ok(())
}
```

or, why not use method chaining?
```rs
Image::open("sample.png")?
    .inverted()
    .save_inferred("inverted.png")?;
```

#### Create a new black image, open the sample image, and paste it on top of the black image:
```rs
let image = Image::new(600, 600, Rgb::black())
    .paste(100, 100, Image::open("sample.png")?);

image.save_inferred("sample_on_black.png")?;
```

you can still use method chaining, but this accesses a lower level interface:
```rs
let image = Image::new(600, 600, Rgb::black())
    .with(&Paste::new(Image::open("sample.png")?).with_position(100, 100))
    .save_inferred("sample_on_black.png")?;
```
