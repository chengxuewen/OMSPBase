//! FFmpeg backend — static libavcodec. Placeholder for Phase 2.

use crate::encoder::{VideoEncoder, EncoderStats};
use crate::decoder::VideoDecoder;
use crate::config::{EncoderConfig, DecoderConfig};
use crate::frame::VideoFrame;
use crate::packet::EncodedPacket;
use crate::error::CodecError;

pub(crate) struct FfmpegEncoder;

impl Default for FfmpegEncoder { fn default() -> Self { Self } }

impl VideoEncoder for FfmpegEncoder {
    fn configure(&mut self, _config: &EncoderConfig) -> Result<(), CodecError> {
        Err(CodecError::Internal("FFmpeg backend not yet implemented".into()))
    }
    fn push_frame(&mut self, _frame: &VideoFrame) -> Result<(), CodecError> {
        Err(CodecError::Internal("FFmpeg backend not yet implemented".into()))
    }
    fn pull_packet(&mut self) -> Result<Option<EncodedPacket>, CodecError> {
        Err(CodecError::Internal("FFmpeg backend not yet implemented".into()))
    }
    fn flush(&mut self) -> Result<(), CodecError> { Ok(()) }
    fn stats(&self) -> EncoderStats { EncoderStats::default() }
}

pub(crate) struct FfmpegDecoder;

impl Default for FfmpegDecoder { fn default() -> Self { Self } }

impl VideoDecoder for FfmpegDecoder {
    fn configure(&mut self, _config: &DecoderConfig) -> Result<(), CodecError> {
        Err(CodecError::Internal("FFmpeg backend not yet implemented".into()))
    }
    fn push_packet(&mut self, _data: &[u8]) -> Result<(), CodecError> { Ok(()) }
    fn pull_frame(&mut self) -> Result<Option<VideoFrame>, CodecError> { Ok(None) }
    fn flush(&mut self) -> Result<(), CodecError> { Ok(()) }
}
