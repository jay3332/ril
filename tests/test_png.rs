use ril::prelude::*;
use std::time::Duration;

pub const COLORS: [Rgb; 12] = [
    Rgb::new(255, 0, 0),
    Rgb::new(255, 128, 0),
    Rgb::new(255, 255, 0),
    Rgb::new(128, 255, 0),
    Rgb::new(0, 255, 0),
    Rgb::new(0, 255, 128),
    Rgb::new(0, 255, 255),
    Rgb::new(0, 128, 255),
    Rgb::new(0, 0, 255),
    Rgb::new(128, 0, 255),
    Rgb::new(255, 0, 255),
    Rgb::new(255, 0, 128),
];

#[test]
fn test_static_png() -> ril::Result<()> {
    let image = Image::<Rgb>::open("tests/sample.png")?;
    assert_eq!(image.dimensions(), (1024, 1024));

    image.save_inferred("tests/out/png_encode_output.png")
}

#[test]
fn test_animated_png_encode() -> ril::Result<()> {
    let mut seq = ImageSequence::new();

    for color in COLORS.into_iter() {
        seq.push_frame(
            Frame::from_image(Image::new(256, 256, color)).with_delay(Duration::from_millis(100)),
        )
    }

    seq.save_inferred("tests/out/apng_encode_output.png")
}

#[test]
fn test_animated_png_decode() -> ril::Result<()> {
    for (frame, ref color) in ImageSequence::<Rgb>::open("tests/apng_sample.png")?.zip(COLORS) {
        let frame = frame?.into_image();

        assert_eq!(frame.dimensions(), (256, 256));
        assert_eq!(frame.pixel(0, 0), color);
    }

    Ok(())
}

#[test]
fn test_paletted_png_encode() -> ril::Result<()> {
    let mut image = Image::<PalettedRgb>::from_paletted_pixels(
        2,
        vec![Rgb::new(255, 255, 255), Rgb::new(0, 0, 0)],
        vec![0, 1, 1, 0, 1, 0, 0, 1, 1, 0],
    );
    // palette mutation test
    let palette = image
        .palette_mut()
        .expect("palette was not registered properly");
    palette[0] = Rgb::new(128, 128, 128);

    assert_eq!(image.pixel(0, 0).color(), Rgb::new(128, 128, 128));
    assert_eq!(image.pixel(1, 0).color(), Rgb::new(0, 0, 0));

    image.save_inferred("tests/out/png_palette_encode_output.png")
}

#[test]
fn test_paletted_png_decode() -> ril::Result<()> {
    let image = Image::<PalettedRgb>::open("tests/palette_sample.png")?;
    assert_eq!(image.dimensions(), (150, 200));
    assert_eq!(image.pixel(0, 0).color(), Rgb::black());
    assert_eq!(image.pixel(100, 100).color(), Rgb::new(200, 8, 8));

    Ok(())
}

#[test]
fn test_palette_mutation() -> ril::Result<()> {
    let mut image = Image::<PalettedRgb>::open("tests/palette_sample.png")?;
    let palette = image
        .palette_mut()
        .expect("palette was not registered properly");
    palette[0] = Rgb::white();

    assert_eq!(image.pixel(0, 0).color(), Rgb::white());
    image.save_inferred("tests/out/png_palette_mutation_output.png")
}

#[test]
fn test_gh_17() -> ril::Result<()> {
    let mut image = Image::<Rgba>::open("tests/sample.png")?;
    let (width, height) = image.dimensions();

    let vertices = vec![(50, 0), (100, 25), (100, 75), (50, 100), (0, 75), (0, 25)];

    let border = Border::<L>::new(L(255), 1);
    let hexagon = Polygon::from_vertices(vertices)
        .with_fill(L(255))
        .with_border(border);

    let mut mask = Image::new(width, height, L(0));

    mask.draw(&hexagon);
    image.mask_alpha(&mask);
    image.save_inferred("sample_hexagon.png")
}

#[test]
fn test_aarch64_simd() -> ril::Result<()> {
    #[allow(clippy::cast_lossless)]
    fn control_merge_impl(original: Rgba, other: Rgba) -> Rgba {
        // Optimize for common cases
        if other.a == 255 {
            return other;
        } else if other.a == 0 {
            return original;
        }

        let (base_r, base_g, base_b, base_a) = (
            original.r as f32 / 255.,
            original.g as f32 / 255.,
            original.b as f32 / 255.,
            original.a as f32 / 255.,
        );

        let (overlay_r, overlay_g, overlay_b, overlay_a) = (
            other.r as f32 / 255.,
            other.g as f32 / 255.,
            other.b as f32 / 255.,
            other.a as f32 / 255.,
        );

        let a_diff = 1. - overlay_a;
        let a = a_diff.mul_add(base_a, overlay_a);

        let a_ratio = a_diff * base_a;
        let r = a_ratio.mul_add(base_r, overlay_a * overlay_r) / a;
        let g = a_ratio.mul_add(base_g, overlay_a * overlay_g) / a;
        let b = a_ratio.mul_add(base_b, overlay_a * overlay_b) / a;

        Rgba {
            r: (r * 255.) as u8,
            g: (g * 255.) as u8,
            b: (b * 255.) as u8,
            a: (a * 255.) as u8,
        }
    }

    unsafe fn simd_merge_impl(base: Rgba, other: Rgba) -> Rgba {
        use std::arch::aarch64::*;

        let base_rgba = vdivq_f32(
            vld1q_f32([base.r as f32, base.g as f32, base.b as f32, base.a as f32].as_ptr()),
            vld1q_dup_f32(&255.0_f32 as *const _),
        );
        let base_a = vgetq_lane_f32::<3>(base_rgba);

        let mask = vdivq_f32(
            vld1q_f32(
                [
                    other.r as f32,
                    other.g as f32,
                    other.b as f32,
                    other.a as f32,
                ]
                .as_ptr(),
            ),
            vld1q_dup_f32(&255.0_f32 as *const _),
        );
        let mask_a = vgetq_lane_f32::<3>(mask);
        let a_diff = 1.0 - mask_a;

        let overlay_rgba = vmulq_f32(
            vsetq_lane_f32::<3>(base_a, mask),
            vsetq_lane_f32::<3>(mask_a, vld1q_dup_f32(&a_diff as *const _)),
        );
        let (overlay_r, overlay_g, overlay_b, a_ratio) =
            std::mem::transmute::<_, (f32, f32, f32, f32)>(overlay_rgba);

        let rgba = vfmaq_f32(
            vld1q_f32([overlay_r, overlay_g, overlay_b, mask_a].as_ptr()),
            vld1q_f32([a_ratio, a_ratio, a_ratio, a_diff].as_ptr()),
            base_rgba,
        );
        let rgba = vmulq_f32(rgba, vld1q_dup_f32(&255.0_f32 as *const _));
        let (r, g, b, a) = std::mem::transmute::<_, (f32, f32, f32, f32)>(rgba);

        Rgba {
            r: r as u8,
            g: g as u8,
            b: b as u8,
            a: a as u8,
        }
    }

    let base = Rgba::new(255, 0, 0, 255);
    let other = Rgba::new(0, 255, 0, 128);

    use std::time::{Duration, Instant};

    let mut total = Duration::from_secs(0);
    for _ in 0..1_000_000 {
        let start = Instant::now();
        control_merge_impl(base, other);
        total += start.elapsed();
    }
    println!("control: {:?}", total);

    let mut total = Duration::from_secs(0);
    for _ in 0..1_000_000 {
        let start = Instant::now();
        unsafe { simd_merge_impl(base, other) };
        total += start.elapsed();
    }
    println!("simd: {:?}", total);

    Ok(())
}
