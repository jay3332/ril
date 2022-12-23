mod aarch64;
mod manual;
mod x86;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use std::is_x86_feature_detected;

pub fn merge_impl() -> unsafe fn(crate::Rgba, crate::Rgba) -> crate::Rgba {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("sse") && is_x86_feature_detected!("fma") {
            return x86::_merge_impl;
        }
    }

    manual::_merge_impl
}
