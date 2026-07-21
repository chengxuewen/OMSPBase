use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rand::Rng;
use rand::SeedableRng;

use super::sink::{SinkId, VideoSink, VideoSinkWants};
use super::source::VideoSource;
use crate::base::buffer::I420Buffer;
use crate::base::buffer::VideoBuffer;
use crate::base::frame::{BoxVideoFrame, VideoFrame};

// ── Bitmap Font ─────────────────────────────────────────

/// Character-cell width in pixels.
const GLYPH_WIDTH: u32 = 6;
/// Character-cell height in pixels.
const GLYPH_HEIGHT: u32 = 10;

/// Stored bitmap for a single glyph — 10 rows of 6 usable bits each.
/// Bits 5–0 map to columns left→right (MSB = leftmost pixel).
/// Bits 7–6 are unused.
struct Glyph(&'static [u8; 10]);

static GLYPHS: [Glyph; 13] = [
    Glyph(&[0x1E, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x1E, 0x00]), // 0
    Glyph(&[0x0C, 0x1C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x3F, 0x00]), // 1
    Glyph(&[0x1E, 0x33, 0x01, 0x03, 0x06, 0x0C, 0x18, 0x30, 0x3F, 0x00]), // 2
    Glyph(&[0x1E, 0x33, 0x01, 0x03, 0x0E, 0x03, 0x01, 0x33, 0x1E, 0x00]), // 3
    Glyph(&[0x03, 0x07, 0x0F, 0x1B, 0x33, 0x3F, 0x03, 0x03, 0x03, 0x00]), // 4
    Glyph(&[0x3F, 0x30, 0x30, 0x3E, 0x03, 0x03, 0x03, 0x33, 0x1E, 0x00]), // 5
    Glyph(&[0x0E, 0x18, 0x30, 0x3E, 0x33, 0x33, 0x33, 0x33, 0x1E, 0x00]), // 6
    Glyph(&[0x3F, 0x03, 0x03, 0x06, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x00]), // 7
    Glyph(&[0x1E, 0x33, 0x33, 0x33, 0x1E, 0x33, 0x33, 0x33, 0x1E, 0x00]), // 8
    Glyph(&[0x1E, 0x33, 0x33, 0x33, 0x1F, 0x03, 0x06, 0x0C, 0x18, 0x00]), // 9
    Glyph(&[0x00, 0x0C, 0x0C, 0x00, 0x00, 0x0C, 0x0C, 0x00, 0x00, 0x00]), // :
    Glyph(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C, 0x00]), // .
    Glyph(&[0x00, 0x00, 0x00, 0x00, 0x3F, 0x00, 0x00, 0x00, 0x00, 0x00]), // -
];

fn glyph_index(ch: char) -> Option<usize> {
    match ch {
        '0'..='9' => Some((ch as usize) - ('0' as usize)),
        ':' => Some(10),
        '.' => Some(11),
        '-' => Some(12),
        _ => None,
    }
}

/// Render one glyph into the Y plane at absolute pixel coordinates `(ox, oy)`.
/// Pixels outside the frame are silently clipped.
fn draw_glyph(
    y: &mut [u8],
    stride_y: usize,
    glyph: &Glyph,
    ox: u32,
    oy: u32,
    width: u32,
    height: u32,
) {
    for row in 0..GLYPH_HEIGHT {
        let py = oy + row;
        if py >= height {
            break;
        }
        let byte = glyph.0[row as usize];
        for col in 0..GLYPH_WIDTH {
            let px = ox + col;
            if px >= width {
                break;
            }
            // ponytail: bit test — GLYPH_WIDTH-1-col maps col 0 to MSB of 6-bit field
            let bit = (byte >> (GLYPH_WIDTH - 1 - col)) & 1;
            if bit == 1 {
                y[py as usize * stride_y + px as usize] = 240; // white
            }
        }
    }
}

/// Render a string of glyphs into the Y plane starting at `(ox, oy)`.
/// Returns the x-offset after the last glyph drawn (for chaining).
fn draw_text(
    y: &mut [u8],
    stride_y: usize,
    text: &str,
    ox: u32,
    oy: u32,
    width: u32,
    height: u32,
) -> u32 {
    let mut cursor_x = ox;
    for ch in text.chars() {
        if let Some(gi) = glyph_index(ch) {
            draw_glyph(y, stride_y, &GLYPHS[gi], cursor_x, oy, width, height);
        }
        cursor_x += GLYPH_WIDTH;
        if cursor_x >= width {
            break;
        }
    }
    cursor_x
}

// ── FramePattern trait ──────────────────────────────────

/// Draws a pattern into pre-allocated I420 plane buffers.
///
/// Implementors are called once per generated frame from the generator thread.
/// The caller guarantees that plane buffers match `width`×`height` and that
/// stride values correctly describe the memory layout.
pub trait FramePattern: Send + Sync {
    /// Fill the three I420 planes with pixel data.
    ///
    /// `y`, `u`, `v` are mutable slices into the respective planes.
    /// `stride_*` gives the row stride (bytes per row) for each plane.
    /// `width` and `height` are the logical frame dimensions.
    #[allow(clippy::too_many_arguments)]
    fn draw(
        &mut self,
        y: &mut [u8],
        u: &mut [u8],
        v: &mut [u8],
        stride_y: usize,
        stride_u: usize,
        stride_v: usize,
        width: u32,
        height: u32,
    );
}

// ── SquarePattern ───────────────────────────────────────

/// A single randomly placed, randomly colored square.
struct Square {
    x: u32,
    y: u32,
    size: u32,
    y_val: u8,
    u_val: u8,
    v_val: u8,
}

/// Generates a pattern of random colored squares plus a bitmap-font timestamp
/// overlay in the top-left corner.
#[allow(dead_code)]
pub struct SquarePattern {
    squares: Vec<Square>,
    width: u32,
    height: u32,
    counter: u64,
}

impl SquarePattern {
    /// Create a new pattern with `num_squares` randomly placed squares.
    ///
    /// Each square has a random position, size (8–63), and YUV color.
    pub fn new(width: u32, height: u32, num_squares: u32) -> Self {
        let mut rng = rand::rngs::StdRng::from_entropy();
        let mut squares = Vec::with_capacity(num_squares as usize);
        for _ in 0..num_squares {
            let size = rng.gen_range(8u32..64);
            // ponytail: keep squares fully within frame
            let max_x = width.saturating_sub(size);
            let max_y = height.saturating_sub(size);
            let x = if max_x > 0 { rng.gen_range(0..max_x) } else { 0 };
            let y = if max_y > 0 { rng.gen_range(0..max_y) } else { 0 };
            squares.push(Square {
                x,
                y,
                size,
                y_val: rng.gen_range(60u8..200),
                u_val: rng.gen_range(80u8..176),
                v_val: rng.gen_range(80u8..176),
            });
        }
        Self {
            squares,
            width,
            height,
            counter: 0,
        }
    }
}

impl FramePattern for SquarePattern {
    fn draw(
        &mut self,
        y: &mut [u8],
        u: &mut [u8],
        v: &mut [u8],
        stride_y: usize,
        stride_u: usize,
        stride_v: usize,
        width: u32,
        height: u32,
    ) {
        // 1. Background fill
        y.fill(16);  // dark gray
        u.fill(128); // gray chroma = no color bias
        v.fill(128);

        // 2. Draw colored squares
        let half_w = width / 2;
        let half_h = height / 2;

        for sq in &self.squares {
            let sx = sq.x;
            let sy = sq.y;
            let sz = sq.size;
            let end_x = (sx + sz).min(width);
            let end_y = (sy + sz).min(height);

            // ponytail: nearest-neighbor fill — simple nested loops
            for row in sy..end_y {
                let y_off = row as usize * stride_y;
                for col in sx..end_x {
                    y[y_off + col as usize] = sq.y_val;
                }
            }

            // UV planes are subsampled 2:1 both dimensions
            let ux = (sx / 2).min(half_w.saturating_sub(1));
            let uy = (sy / 2).min(half_h.saturating_sub(1));
            let u_end_x = ((end_x / 2).min(half_w)).max(ux + 1);
            let u_end_y = ((end_y / 2).min(half_h)).max(uy + 1);

            for row in uy..u_end_y {
                let u_off = row as usize * stride_u;
                let v_off = row as usize * stride_v;
                for col in ux..u_end_x {
                    u[u_off + col as usize] = sq.u_val;
                    v[v_off + col as usize] = sq.v_val;
                }
            }
        }

        // 3. Timestamp overlay — render counter as text at top-left
        self.counter += 1;
        let text = format!("{:05}", self.counter % 100_000);
        draw_text(
            y,
            stride_y,
            &text,
            4,  // left margin
            4,  // top margin
            width,
            height,
        );
    }
}

// ── VideoFrameGenerator ─────────────────────────────────

/// A video source that generates frames on a dedicated thread.
///
/// Each frame uses a [`FramePattern`] to paint into an [`I420Buffer`],
/// which is then wrapped in a [`BoxVideoFrame`] and broadcast to all
/// registered sinks.
pub struct VideoFrameGenerator {
    #[allow(clippy::type_complexity)]
    sinks: Arc<Mutex<Vec<(SinkId, Box<dyn VideoSink<BoxVideoFrame>>, VideoSinkWants)>>>,
    running: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
    next_id: AtomicU64,
}

impl Default for VideoFrameGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoFrameGenerator {
    pub fn new() -> Self {
        Self {
            sinks: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
            thread: Mutex::new(None),
            next_id: AtomicU64::new(0),
        }
    }

    /// Start the generation thread.
    ///
    /// Spawns a new OS thread that:
    /// 1. Creates an [`I420Buffer`] each iteration.
    /// 2. Calls `pattern.draw()` to fill it.
    /// 3. Wraps the buffer in a [`BoxVideoFrame`] with a monotonic timestamp.
    /// 4. Broadcasts the frame to all active sinks.
    /// 5. Sleeps for the frame interval, with drift compensation.
    ///
    /// # Panics
    ///
    /// Panics if the generator is already running (check via `is_running()` first).
    pub fn start(&self, fps: u32, mut pattern: Box<dyn FramePattern>, width: u32, height: u32) {
        let mut guard = self.thread.lock().unwrap();
        if guard.is_some() || self.running.load(Ordering::Relaxed) {
            drop(guard);
            panic!("VideoFrameGenerator already running");
        }

        let sinks = Arc::clone(&self.sinks);
        let running = Arc::clone(&self.running);
        self.running.store(true, Ordering::SeqCst);

        let handle = thread::Builder::new()
            .name("omsp-video-gen".into())
            .spawn(move || {
                let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);
                let mut next_frame_time = Instant::now();
                let mut frame_id: u32 = 0;

                while running.load(Ordering::Relaxed) {
                    // ── Create frame ─────────────────
                    let mut buf = I420Buffer::new(width, height);
                    let bw = buf.width();
                    let bh = buf.height();
                    pattern.draw(
                        &mut buf.data_y,
                        &mut buf.data_u,
                        &mut buf.data_v,
                        buf.stride_y as usize,
                        buf.stride_u as usize,
                        buf.stride_v as usize,
                        bw,
                        bh,
                    );

                    let vb: Box<dyn VideoBuffer> = Box::new(buf);
                    let frame = VideoFrame::new(vb)
                        .with_timestamp(timestamp_us());
                    frame_id = frame_id.wrapping_add(1);

                    // ── Broadcast ────────────────────
                    if let Ok(guard) = sinks.lock() {
                        for (_id, sink, wants) in guard.iter() {
                            if wants.is_active {
                                // ponytail: ignore per-sink errors; a bad sink
                                // shouldn't kill the generator thread
                                let _ = sink.on_frame(&frame);
                            }
                        }
                    }

                    // ── Frame pacing with drift compensation
                    let now = Instant::now();
                    if now < next_frame_time {
                        thread::sleep(next_frame_time - now);
                    }
                    next_frame_time += frame_duration;

                    // ponytail: reset clock if we fell behind by more than one frame
                    if next_frame_time <= Instant::now() {
                        next_frame_time = Instant::now() + frame_duration;
                    }
                }
            })
            .expect("failed to spawn generator thread");

        *guard = Some(handle);
    }

    /// Stop the generation thread and wait for it to exit.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        if let Ok(mut guard) = self.thread.lock()
            && let Some(handle) = guard.take()
        {
            drop(guard);
            // ponytail: join outside the lock to avoid potential deadlock
            let _ = handle.join();
        }
    }

    /// Check whether the generator thread is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Number of currently registered sinks.
    pub fn sink_count(&self) -> usize {
        self.sinks.lock().map(|g| g.len()).unwrap_or(0)
    }
}

impl VideoSource<BoxVideoFrame> for VideoFrameGenerator {
    fn add_or_update_sink(
        &self,
        sink: Box<dyn VideoSink<BoxVideoFrame>>,
        wants: VideoSinkWants,
    ) -> SinkId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut guard) = self.sinks.lock() {
            guard.push((id, sink, wants));
        }
        id
    }

    fn remove_sink(&self, id: SinkId) {
        if let Ok(mut guard) = self.sinks.lock() {
            guard.retain(|(sid, _, _)| *sid != id);
        }
    }
}

// ── Helpers ─────────────────────────────────────────────

/// Monotonic timestamp in microseconds.
fn timestamp_us() -> i64 {
    // ponytail: use Instant-based monotonic counter instead of wall clock
    use std::sync::atomic::AtomicI64;
    use std::time::UNIX_EPOCH;
    static BASE: AtomicI64 = AtomicI64::new(-1);
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

    let start = START.get_or_init(Instant::now);
    if BASE.load(Ordering::Relaxed) < 0 {
        let sys_us = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_micros() as i64)
            .unwrap_or(0);
        BASE.store(sys_us, Ordering::Relaxed);
    }
    BASE.load(Ordering::Relaxed) + start.elapsed().as_micros() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use crate::base::frame::BoxVideoFrame;
    use crate::error::MediaError;
    use crate::pipeline::sink::{VideoSink, VideoSinkWants};

    /// Minimal pattern that just fills planes with gray.
    struct GrayPattern;

    impl FramePattern for GrayPattern {
        fn draw(
            &mut self,
            y: &mut [u8],
            u: &mut [u8],
            v: &mut [u8],
            _stride_y: usize,
            _stride_u: usize,
            _stride_v: usize,
            _width: u32,
            _height: u32,
        ) {
            y.fill(128);
            u.fill(128);
            v.fill(128);
        }
    }

    /// Sink that records how many frames it received.
    struct CountingSink {
        count: Arc<Mutex<u32>>,
        wants: VideoSinkWants,
    }

    impl VideoSink<BoxVideoFrame> for CountingSink {
        fn on_frame(&self, _frame: &BoxVideoFrame) -> Result<VideoSinkWants, MediaError> {
            *self.count.lock().unwrap() += 1;
            Ok(self.wants)
        }
    }

    #[test]
    fn generator_produces_frames_within_timeout() {
        let generator = VideoFrameGenerator::new();
        assert!(!generator.is_running());
        assert_eq!(generator.sink_count(), 0);

        let count = Arc::new(Mutex::new(0u32));
        let sink = Box::new(CountingSink {
            count: count.clone(),
            wants: VideoSinkWants::default(),
        });
        generator.add_or_update_sink(sink, VideoSinkWants::default());
        assert_eq!(generator.sink_count(), 1);

        // Start at 60 fps with a 16×16 frame
        generator.start(60, Box::new(GrayPattern), 16, 16);
        assert!(generator.is_running());

        // Wait 200ms — at 60fps we expect ~12 frames
        std::thread::sleep(Duration::from_millis(200));

        generator.stop();
        assert!(!generator.is_running());

        let frames = *count.lock().unwrap();
        // ponytail: scheduler jitter means we might get fewer frames,
        // but anything >= 5 in 200ms proves the generator works
        assert!(
            frames >= 5,
            "expected at least 5 frames in 200ms at 60fps, got {}", frames
        );
    }

    #[test]
    #[should_panic(expected = "already running")]
    fn generator_double_start_panics() {
        let generator = VideoFrameGenerator::new();
        generator.start(10, Box::new(GrayPattern), 4, 4);
        // Second start should panic
        generator.start(10, Box::new(GrayPattern), 4, 4);
    }
}
