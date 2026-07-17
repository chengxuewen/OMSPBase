// GStreamer decode pipeline — receive WebRTC stream, decode via decodebin, render
// Feature-gated behind `gstreamer` feature

use std::error::Error;

/// GStreamer decodebin pipeline for receiving and rendering a remote stream
#[cfg(feature = "gstreamer")]
pub struct DecodePipeline {
    pipeline: Option<gstreamer::Pipeline>,
    display: String,
    width: u32,
    height: u32,
    decoder: Option<String>,
}

#[cfg(feature = "gstreamer")]
impl DecodePipeline {
    /// Build a receiver pipeline:
    ///   decodebin → videoconvert → videoscale → autovideosink
    ///
    /// In production, the decodebin input would be a webrtcbin pad or appsrc fed from
    /// the WebRTC data path. For MVP, pipeline is a template with placeholders.
    pub fn new(display: &str, width: u32, height: u32, decoder: Option<&str>) -> Self {
        DecodePipeline {
            pipeline: None,
            display: display.to_string(),
            width,
            height,
            decoder: decoder.map(String::from),
        }
    }

    /// Build and start the GStreamer pipeline
    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        // ponytail: placeholder pipeline — real webrtcbin integration comes with full WebRTC
        let desc = format!(
            "appsrc name=source ! decodebin ! videoconvert ! videoscale ! video/x-raw,width={},height={} ! autovideosink sync=false",
            self.width, self.height,
        );
        if let Some(ref dec) = self.decoder {
            tracing::info!("Remote decoder requested: {dec}");
        }

        let pipeline = gstreamer::parse::launch(&desc)?
            .downcast::<gstreamer::Pipeline>()
            .map_err(|_| "Failed to create decode pipeline")?;

        pipeline.set_state(gstreamer::State::Playing)?;
        self.pipeline = Some(pipeline);
        tracing::info!("Decode pipeline started for {display}", display = self.display);
        Ok(())
    }

    /// Stop the pipeline
    pub fn stop(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(ref pipeline) = self.pipeline {
            pipeline.set_state(gstreamer::State::Null)?;
            tracing::info!("Decode pipeline stopped");
        }
        self.pipeline = None;
        Ok(())
    }
}

#[cfg(feature = "gstreamer")]
impl Drop for DecodePipeline {
    fn drop(&mut self) {
        if let Some(ref pipeline) = self.pipeline {
            let _ = pipeline.set_state(gstreamer::State::Null);
        }
    }
}

/// No-op pipeline stub when gstreamer feature is disabled
#[cfg(not(feature = "gstreamer"))]
pub struct DecodePipeline;

#[cfg(not(feature = "gstreamer"))]
impl DecodePipeline {
    pub fn new(_display: &str, _width: u32, _height: u32, _decoder: Option<&str>) -> Self {
        tracing::info!("Decode pipeline disabled (gstreamer feature not enabled)");
        DecodePipeline
    }

    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        tracing::warn!("GStreamer decode not available — build with --features gstreamer");
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
