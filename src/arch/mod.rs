#![allow(clippy::cast_lossless)]
#![allow(clippy::wildcard_imports)]

mod aarch64;
mod manual;
mod x86;

use crate::Rgba;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use std::is_x86_feature_detected;

#[inline]
pub fn merge_impl(base: Rgba, other: Rgba) -> Rgba {
    // Optimize for common cases
    if other.a == 255 {
        return other;
    } else if other.a == 0 {
        return base;
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if is_x86_feature_detected!("sse") && is_x86_feature_detected!("fma") {
        unsafe {
            return x86::_merge_impl(base, other);
        }
    }

    manual::_merge_impl(base, other)
}
