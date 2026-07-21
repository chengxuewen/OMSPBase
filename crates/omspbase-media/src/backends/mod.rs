#[cfg(all(feature = "backend-libyuv-sys", feature = "backend-native"))]
compile_error!("Only one backend can be enabled at a time.");

#[cfg(feature = "backend-libyuv-sys")]
mod libyuv;
#[cfg(feature = "backend-libyuv-sys")]
pub use libyuv::LibyuvTransform;

#[cfg(feature = "backend-native")]
mod native;
#[cfg(feature = "backend-native")]
pub use native::NativeTransform;

use crate::transform::VideoTransform;

/// Returns the compiled backend. Zero dynamic dispatch.
#[cfg(feature = "backend-libyuv-sys")]
pub fn get_backend() -> impl VideoTransform {
    LibyuvTransform
}

#[cfg(all(feature = "backend-native", not(feature = "backend-libyuv-sys")))]
pub fn get_backend() -> impl VideoTransform {
    NativeTransform
}