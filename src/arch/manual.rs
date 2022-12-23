use crate::Rgba;

pub fn _merge_impl(base: Rgba, other: Rgba) -> Rgba {
    let (base_r, base_g, base_b, base_a) = (
        base.r as f32 / 255.,
        base.g as f32 / 255.,
        base.b as f32 / 255.,
        base.a as f32 / 255.,
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

#[inline]
pub fn _invert_impl(base: Rgba) -> Rgba {
    Rgba {
        r: !base.r,
        g: !base.g,
        b: !base.b,
        a: !base.a,
    }
}
