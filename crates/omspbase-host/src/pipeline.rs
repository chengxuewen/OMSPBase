// GStreamer pipeline: V4L2 capture → NVENC encode → appsink

#[cfg(feature = "gstreamer")]
mod imp {
    use gstreamer::prelude::*;
    use gstreamer_app;

    pub struct Pipeline {
        pipeline: gstreamer::Pipeline,
        appsink: gstreamer_app::AppSink,
    }

    impl Pipeline {
        pub fn new(camera: &str, width: u32, height: u32, fps: u32, bitrate: u32, encoder: &str) -> Result<Self, Box<dyn std::error::Error>> {
            let pipeline_desc = format!(
                "v4l2src device={} ! videoconvert ! video/x-raw,format=I420,width={},height={},framerate={}/1 ! {} bitrate={} ! h264parse ! appsink name=sink",
                camera, width, height, fps, encoder, bitrate
            );
            let pipeline = gstreamer::parse::launch(&pipeline_desc)?
                .downcast::<gstreamer::Pipeline>()
                .map_err(|_| "Failed to create pipeline")?;

            let appsink = pipeline
                .by_name("sink")
                .ok_or("appsink 'sink' not found in pipeline")?
                .downcast::<gstreamer_app::AppSink>()
                .map_err(|_| "element 'sink' is not an AppSink")?;

            // Set up bus watch for error/warning logging (recovery in next sub-task)
            let bus = pipeline.bus().ok_or("pipeline has no bus")?;
            bus.set_sync_handler(move |_bus, msg| {
                use gstreamer::MessageView;
                match msg.view() {
                    MessageView::Error(err) => {
                        tracing::error!(
                            src = ?err.src(),
                            error = %err.error(),
                            debug = err.debug(),
                            "GStreamer pipeline error"
                        );
                    }
                    MessageView::Warning(warn) => {
                        tracing::warn!(
                            src = ?warn.src(),
                            error = %warn.error(),
                            debug = warn.debug(),
                            "GStreamer pipeline warning"
                        );
                    }
                    MessageView::Eos(_) => {
                        tracing::info!("GStreamer pipeline EOS");
                    }
                    _ => {}
                }
                gstreamer::BusSyncReply::Pass
            });

            Ok(Pipeline { pipeline, appsink })
        }

        pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
            self.pipeline.set_state(gstreamer::State::Playing)?;
            Ok(())
        }

        pub fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
            self.pipeline.set_state(gstreamer::State::Null)?;
            Ok(())
        }

        pub fn pull_frame(&self) -> Option<Vec<u8>> {
            let sample = self.appsink.try_pull_sample(gstreamer::ClockTime::NONE).ok()??;
            let buffer = sample.buffer()?;
            let map = buffer.map_readable().ok()?;
            Some(map.to_vec())
        }
    }
}

#[cfg(not(feature = "gstreamer"))]
mod imp {
    pub struct Pipeline;

    impl Pipeline {
        pub fn new(_camera: &str, _width: u32, _height: u32, _fps: u32, _bitrate: u32, _encoder: &str) -> Result<Self, Box<dyn std::error::Error>> {
            tracing::warn!("Pipeline stub (no GStreamer compiled)");
            Ok(Pipeline)
        }

        pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }

        pub fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }

        pub fn pull_frame(&self) -> Option<Vec<u8>> {
            None
        }
    }
}

pub use imp::Pipeline;
