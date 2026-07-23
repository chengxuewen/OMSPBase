//! Adapters that wrap remote crate types as omspbase_common pipeline nodes,
//! so they can be orchestrated by PipelineEngine.
//!
//! - FrameSource: wraps tokio::sync::mpsc::UnboundedReceiver<Vec<u8>> as MediaSource
//! - DecodeSink: wraps crate::decode::DecodePipeline as MediaSink

use std::sync::Arc;

use omspbase_common::error::CoreError;
use omspbase_media::pipeline::core::{
    EncodedFragment, FormatSpec, FragmentFlags, FrameTiming, InternalPacket, MediaSink,
    MediaSource, MediaType, NodeCapability, NodeInfo, PipelineNode,
};

type Result<T> = std::result::Result<T, CoreError>;

// ── FrameSource ──

/// Wraps an unbounded MPSC receiver for frames arriving from WebRTC transport.
/// Yields encoded H.264 byte-stream fragments to the pipeline.
pub struct FrameSource {
    rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
}

impl FrameSource {
    pub fn new(rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>) -> Self {
        Self { rx }
    }
}

impl NodeInfo for FrameSource {
    fn name(&self) -> &str {
        "webrtc-source"
    }

    fn capabilities(&self) -> NodeCapability {
        NodeCapability {
            input: FormatSpec {
                media_type: MediaType::Both,
                codecs: None,
                pixel_formats: vec![],
            },
            output: FormatSpec {
                media_type: MediaType::Encoded,
                codecs: None,
                pixel_formats: vec![],
            },
        }
    }
}

impl PipelineNode for FrameSource {
    fn on_start(&mut self) -> Result<()> {
        Ok(())
    }

    fn on_stop(&mut self) -> Result<()> {
        Ok(())
    }
}

impl MediaSource for FrameSource {
    type Output = InternalPacket;

    fn poll_fragment(&mut self) -> Result<Option<Self::Output>> {
        match self.rx.try_recv() {
            Ok(data) => Ok(Some(InternalPacket::Encoded(EncodedFragment {
                track_id: "webrtc".into(),
                timing: FrameTiming {
                    dts: 0,
                    pts: 0,
                    duration: 0,
                    wall_clock: Some(std::time::Instant::now()),
                },
                flags: FragmentFlags {
                    keyframe: false,
                    independent: true,
                    discardable: false,
                },
                codec: "h264".into(),
                init_data: None,
                payload: data,
            }))),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                tracing::info!("FrameSource: channel disconnected");
                Ok(None)
            }
        }
    }
}

// ── DecodeSink ──

/// Wraps DecodePipeline as an engine sink: receives encoded packets from
/// the pipeline and feeds them into the GStreamer decode pipeline.
pub struct DecodeSink {
    pipeline: Arc<crate::decode::DecodePipeline>,
}

impl DecodeSink {
    pub fn new(pipeline: Arc<crate::decode::DecodePipeline>) -> Self {
        Self { pipeline }
    }
}

impl NodeInfo for DecodeSink {
    fn name(&self) -> &str {
        "decode-sink"
    }

    fn capabilities(&self) -> NodeCapability {
        NodeCapability {
            input: FormatSpec {
                media_type: MediaType::Encoded,
                codecs: None,
                pixel_formats: vec![],
            },
            output: FormatSpec {
                media_type: MediaType::Encoded,
                codecs: None,
                pixel_formats: vec![],
            },
        }
    }
}

impl PipelineNode for DecodeSink {
    fn on_start(&mut self) -> Result<()> {
        Ok(())
    }

    fn on_stop(&mut self) -> Result<()> {
        Ok(())
    }
}

impl MediaSink for DecodeSink {
    type Input = InternalPacket;

    fn on_fragment(&mut self, fragment: Self::Input) -> Result<()> {
        let payload = match fragment {
            InternalPacket::Encoded(f) => f.payload,
            _ => return Ok(()),
        };
        self.pipeline.push_h264(&payload)
    }
}
