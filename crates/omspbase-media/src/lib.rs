//! OMSPBase Media — video frame types, buffers, source/sink pipeline, transforms.

pub mod error;
pub mod pixel_format;
pub mod transform;
pub mod base;
pub mod pipeline;
pub mod backends;

#[cfg(not(any(feature = "backend-libyuv-sys", feature = "backend-native")))]
compile_error!(
    "At least one backend feature must be enabled: \
     backend-libyuv-sys or backend-native"
);