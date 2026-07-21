//! OMSPBase SquareGenerator Demo with egui
//!
//! Usage:
//!   cargo run -p omspbase-media --example square-gen-egui
//!
//! Creates a VideoFrameGenerator (colored squares + timestamp),
//! implements VideoSink to capture frames, and displays them in an egui window.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use eframe::egui;
#[cfg(feature = "backend-native")]
use omspbase_media::backends::NativeTransform;
#[cfg(feature = "backend-libyuv-sys")]
use omspbase_media::backends::LibyuvTransform;
use omspbase_media::base::frame::BoxVideoFrame;
use omspbase_media::error::MediaError;
use omspbase_media::pipeline::generator::{SquarePattern, VideoFrameGenerator};
use omspbase_media::pipeline::sink::{VideoSink, VideoSinkWants};
use omspbase_media::pipeline::source::VideoSource;
use omspbase_media::pixel_format::PixelFormat;
use omspbase_media::transform::VideoTransform;
const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const FPS: u32 = 30;

struct FrameQueue {
    data: Mutex<VecDeque<Vec<u8>>>,
}

impl FrameQueue {
    fn new() -> Self {
        Self {
            data: Mutex::new(VecDeque::new()),
        }
    }

    fn push(&self, rgba: Vec<u8>) {
        let mut q = self.data.lock().unwrap();
        // ponytail: keep at most 3 frames in queue, drop oldest
        if q.len() > 2 {
            q.pop_front();
        }
        q.push_back(rgba);
    }

    fn take_latest(&self) -> Option<Vec<u8>> {
        let mut q = self.data.lock().unwrap();
        let latest = q.pop_back();
        q.clear();
        latest
    }
}

/// Bridges VideoFrameGenerator (I420) to an Arc<FrameQueue> of RGBA data.
struct GeneratorSink {
    queue: Arc<FrameQueue>,
}

impl VideoSink<BoxVideoFrame> for GeneratorSink {
    fn on_frame(&self, frame: &BoxVideoFrame) -> Result<VideoSinkWants, MediaError> {
        if let Some(i420_ref) = frame.buffer.as_i420_ref() {
            let mut rgba = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
            #[cfg(feature = "backend-libyuv-sys")]
            let result = LibyuvTransform::i420_to_argb(i420_ref, WIDTH, HEIGHT, PixelFormat::RGBA, &mut rgba);
#[cfg(all(feature = "backend-native", not(feature = "backend-libyuv-sys")))]
            let result = NativeTransform::i420_to_argb(i420_ref, WIDTH, HEIGHT, PixelFormat::RGBA, &mut rgba);
            result.expect("i420_to_argb failed");
            self.queue.push(rgba);
        }
        Ok(VideoSinkWants::default())
    }
}

struct App {
    generator: VideoFrameGenerator,
    queue: Arc<FrameQueue>,
    running: bool,
    frame_count: u64,
    texture: Option<egui::TextureHandle>,
}

impl App {
    fn new() -> Self {
        let generator = VideoFrameGenerator::new();
        let queue = Arc::new(FrameQueue::new());
        generator.add_or_update_sink(
            Box::new(GeneratorSink {
                queue: queue.clone(),
            }),
            VideoSinkWants {
                is_active: true,
                ..Default::default()
            },
        );
        Self {
            generator,
            queue,
            running: false,
            frame_count: 0,
            texture: None,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if self.running {
                    if ui.button("\u{25A0} Stop").clicked() {
                        self.generator.stop();
                        self.running = false;
                    }
                } else if ui.button("\u{25B6} Start").clicked() {
                    self.generator.start(
                        FPS,
                        Box::new(SquarePattern::new(WIDTH, HEIGHT, 50)),
                        WIDTH,
                        HEIGHT,
                    );
                    self.running = true;
                }
                ui.separator();
                ui.label(format!(
                    "{}x{}  {}fps  Frame: {}",
                    WIDTH, HEIGHT, FPS, self.frame_count
                ));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.running {
                if let Some(rgba) = self.queue.take_latest() {
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        [WIDTH as usize, HEIGHT as usize],
                        &rgba,
                    );
                    let tex = ctx.load_texture(
                        "frame",
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.texture = Some(tex);
                    self.frame_count += 1;
                }
            }

            if let Some(tex) = &self.texture {
                let available = ui.available_size();
                let scale = (available.x / WIDTH as f32)
                    .min(available.y / HEIGHT as f32)
                    .min(1.0);
                ui.image(egui::load::SizedTexture::new(
                    tex.id(),
                    [WIDTH as f32 * scale, HEIGHT as f32 * scale],
                ));
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Press Start to begin");
                });
            }

            if self.running {
                ctx.request_repaint();
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    eframe::run_native(
        "OMSPBase SquareGenerator Demo",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}
