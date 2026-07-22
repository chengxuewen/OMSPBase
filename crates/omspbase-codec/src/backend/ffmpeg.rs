//! FFmpeg backend — static libavcodec. Placeholder (no-op) for now.
//! Real implementation via ffmpeg-the-third crate deferred to Phase 2.

use crate::encoder::{VideoEncoder, EncoderStats};
use crate::decoder::VideoDecoder;
use crate::config::{EncoderConfig, DecoderConfig};
use crate::frame::VideoFrame;
use crate::packet::EncodedPacket;
use crate::error::CodecError;

pub(crate) struct FfmpegEncoder;
impl Default for FfmpegEncoder { fn default() -> Self { Self } }

impl VideoEncoder for FfmpegEncoder {
    fn configure(&mut self, _: &EncoderConfig) -> Result<(), CodecError> { Ok(()) }
    fn push_frame(&mut self, _: &VideoFrame) -> Result<(), CodecError> { Ok(()) }
    fn pull_packet(&mut self) -> Result<Option<EncodedPacket>, CodecError> { Ok(None) }
    fn flush(&mut self) -> Result<(), CodecError> { Ok(()) }
    fn stats(&self) -> EncoderStats { EncoderStats::default() }
}

pub(crate) struct FfmpegDecoder;
impl Default for FfmpegDecoder { fn default() -> Self { Self } }

impl VideoDecoder for FfmpegDecoder {
    fn configure(&mut self, _: &DecoderConfig) -> Result<(), CodecError> { Ok(()) }
    fn push_packet(&mut self, _: &[u8]) -> Result<(), CodecError> { Ok(()) }
    fn pull_frame(&mut self) -> Result<Option<VideoFrame>, CodecError> { Ok(None) }
    fn flush(&mut self) -> Result<(), CodecError> { Ok(()) }
}
