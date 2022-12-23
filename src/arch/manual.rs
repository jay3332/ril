use crate::Rgba;

#[allow(clippy::cast_lossless)]
pub fn _merge_impl(original: Rgba, other: Rgba) -> Rgba {
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
