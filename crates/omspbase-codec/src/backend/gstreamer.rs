//! GStreamer backend — dynamic .so via gstreamer crate. Placeholder for Phase 3.

use crate::encoder::{VideoEncoder, EncoderStats};
use crate::decoder::VideoDecoder;
use crate::config::{EncoderConfig, DecoderConfig};
use crate::frame::VideoFrame;
use crate::packet::EncodedPacket;
use crate::error::CodecError;

pub(crate) struct GstEncoder;

impl Default for GstEncoder { fn default() -> Self { Self } }

impl VideoEncoder for GstEncoder {
    fn configure(&mut self, _config: &EncoderConfig) -> Result<(), CodecError> { Ok(()) }
    fn push_frame(&mut self, _frame: &VideoFrame) -> Result<(), CodecError> { Ok(()) }
    fn pull_packet(&mut self) -> Result<Option<EncodedPacket>, CodecError> { Ok(None) }
    fn flush(&mut self) -> Result<(), CodecError> { Ok(()) }
    fn stats(&self) -> EncoderStats { EncoderStats::default() }
}

pub(crate) struct GstDecoder;

impl Default for GstDecoder { fn default() -> Self { Self } }

impl VideoDecoder for GstDecoder {
    fn configure(&mut self, _config: &DecoderConfig) -> Result<(), CodecError> { Ok(()) }
    fn push_packet(&mut self, _data: &[u8]) -> Result<(), CodecError> { Ok(()) }
    fn pull_frame(&mut self) -> Result<Option<VideoFrame>, CodecError> { Ok(None) }
    fn flush(&mut self) -> Result<(), CodecError> { Ok(()) }
}
