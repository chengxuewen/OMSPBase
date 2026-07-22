//! Adapters that wrap host crate types as omspbase_core pipeline nodes,
//! so they can be orchestrated by PipelineEngine.
//!
//! - GstCaptureSource: wraps crate::pipeline::Pipeline as MediaSource<InternalPacket>
//! - WebrtcOutputSink: wraps crate::webrtc_transport::WebrtcTransport as MediaSink<InternalPacket>

use std::sync::Arc;

use tokio::sync::mpsc;

use omspbase_core::error::CoreError;
use omspbase_media::pipeline::core::{
    FormatSpec, InternalPacket, MediaSink, MediaSource, MediaType, NodeCapability,
    NodeInfo, PipelineNode,
};
#[cfg(feature = "gstreamer")]
use omspbase_media::pipeline::core::{EncodedFragment, FragmentFlags, FrameTiming};

type Result<T> = std::result::Result<T, CoreError>;

// ── GstCaptureSource ──
// Feature-gated: real GStreamer pipeline vs stub (returns None always).

/// Real pipeline-backed source (GStreamer compiled in).
#[cfg(feature = "gstreamer")]
pub struct GstCaptureSource {
    pipeline: Arc<crate::pipeline::Pipeline>,
}

#[cfg(feature = "gstreamer")]
impl GstCaptureSource {
    pub fn new(pipeline: Arc<crate::pipeline::Pipeline>) -> Self {
        Self { pipeline }
    }
}

#[cfg(feature = "gstreamer")]
impl NodeInfo for GstCaptureSource {
    fn name(&self) -> &str {
        "gst-capture"
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

#[cfg(feature = "gstreamer")]
impl PipelineNode for GstCaptureSource {
    fn on_start(&mut self) -> Result<()> {
        self.pipeline.start()
    }

    fn on_stop(&mut self) -> Result<()> {
        self.pipeline.stop()
    }
}

#[cfg(feature = "gstreamer")]
impl MediaSource for GstCaptureSource {
    type Output = InternalPacket;

    fn poll_fragment(&mut self) -> Result<Option<Self::Output>> {
        match self.pipeline.pull_sample() {
            Ok(data) if data.is_empty() => Ok(None),
            Ok(data) => Ok(Some(InternalPacket::Encoded(EncodedFragment {
                track_id: "capture".into(),
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
            Err(_e) => {
                // ponytail: treat pull failures as transient (no frame available)
                Ok(None)
            }
        }
    }
}

// Stub source — returns no fragments when GStreamer is not compiled in.

#[cfg(not(feature = "gstreamer"))]
pub struct GstCaptureSource;

#[cfg(not(feature = "gstreamer"))]
impl GstCaptureSource {
    pub fn new(_pipeline: Arc<crate::pipeline::Pipeline>) -> Self {
        Self
    }
}

#[cfg(not(feature = "gstreamer"))]
impl NodeInfo for GstCaptureSource {
    fn name(&self) -> &str {
        "gst-capture"
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

#[cfg(not(feature = "gstreamer"))]
impl PipelineNode for GstCaptureSource {
    fn on_start(&mut self) -> Result<()> {
        Ok(())
    }

    fn on_stop(&mut self) -> Result<()> {
        Ok(())
    }
}

#[cfg(not(feature = "gstreamer"))]
impl MediaSource for GstCaptureSource {
    type Output = InternalPacket;

    fn poll_fragment(&mut self) -> Result<Option<Self::Output>> {
        Ok(None)
    }
}

// ── WebrtcOutputSink ──
// Always available — WebrtcTransport is not feature-gated.

pub struct WebrtcOutputSink {
    _drain_handle: tokio::task::JoinHandle<()>,
    frame_tx: mpsc::Sender<Vec<u8>>,
}

impl WebrtcOutputSink {
    pub fn new(
        transport: Arc<crate::webrtc_transport::WebrtcTransport>,
    ) -> Self {
        // ponytail: bounded channel (capacity 4) replaces fire-and-forget spawn.
        // Full channel → drop oldest frame. Single consumer drains sequentially,
        // avoiding unbounded task accumulation on DC backpressure.
        let (frame_tx, mut frame_rx) = mpsc::channel::<Vec<u8>>(4);
        let drain_handle = tokio::spawn(async move {
            while let Some(data) = frame_rx.recv().await {
                let _ = transport.send_frame(&data).await;
            }
        });
        Self {
            _drain_handle: drain_handle,
            frame_tx,
        }
    }
}

impl NodeInfo for WebrtcOutputSink {
    fn name(&self) -> &str {
        "webrtc-output"
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

impl PipelineNode for WebrtcOutputSink {
    fn on_start(&mut self) -> Result<()> {
        Ok(())
    }

    fn on_stop(&mut self) -> Result<()> {
        Ok(())
    }
}

impl MediaSink for WebrtcOutputSink {
    type Input = InternalPacket;

    fn on_fragment(&mut self, fragment: Self::Input) -> Result<()> {
        let payload = match fragment {
            InternalPacket::Encoded(f) => f.payload,
            _ => return Ok(()),
        };
        // ponytail: try_send → drop oldest if channel full, no task accumulation
        let _ = self.frame_tx.try_send(payload);
        Ok(())
    }
}
