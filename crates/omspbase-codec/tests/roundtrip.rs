//! Roundtrip tests — I420 → H.264 encode → I420 decode.
//! Requires FFmpeg backend (feature = "backend-ffmpeg").

use omspbase_codec::codec::{CodecId, PixelFormat, VideoFormat};
use omspbase_codec::config::{Bitrate, EncoderConfig, DecoderConfig, EncoderPreset};
use omspbase_codec::encoder::VideoEncoder;
use omspbase_codec::decoder::VideoDecoder;
use omspbase_codec::factory::CodecFactory;
use omspbase_codec::frame::{VideoFrame, Plane};

fn make_test_config(w: u32, h: u32) -> EncoderConfig {
    let fmt = VideoFormat { width: w, height: h, pixel_format: PixelFormat::Yuv420p };
    EncoderConfig::builder(CodecId::H264, fmt)
        .bitrate(Bitrate::Cbr(2000))
        .fps(30, 1)
        .preset(EncoderPreset::P4Medium)
        .gop(30)
        .build()
}

fn make_gradient_frame(w: u32, h: u32) -> VideoFrame {
    let y_size = (w * h) as usize;
    let uv_size = ((w / 2) * (h / 2)) as usize;

    let mut y = vec![0u8; y_size];
    for row in 0..h {
        let val = (row * 256 / h) as u8;
        for col in 0..w {
            y[(row * w + col) as usize] = val;
        }
    }

    let u = vec![128u8; uv_size];
    let v = vec![128u8; uv_size];

    VideoFrame {
        format: VideoFormat { width: w, height: h, pixel_format: PixelFormat::Yuv420p },
        planes: vec![
            Plane { data: y, stride: w },
            Plane { data: u, stride: w / 2 },
            Plane { data: v, stride: w / 2 },
        ],
        pts: 0,
        keyframe: true,
    }
}

fn compute_psnr(original: &[u8], decoded: &[u8]) -> f64 {
    assert_eq!(original.len(), decoded.len());
    let mse = original.iter().zip(decoded.iter())
        .map(|(a, b)| (*a as f64 - *b as f64).powi(2))
        .sum::<f64>() / original.len() as f64;
    if mse < 0.0001 { return 100.0; }
    10.0 * (255.0_f64.powi(2) / mse).log10()
}

#[test]
fn roundtrip_i420_h264_i420_dimensions_preserved() {
    let factory = CodecFactory::new();
    let cfg = make_test_config(320, 240);
    let mut encoder = factory.create_encoder(cfg.clone(), None).unwrap();
    encoder.configure(&cfg).unwrap();
    let mut decoder = factory.create_decoder(
        DecoderConfig { codec: CodecId::H264 }, None,
    ).unwrap();

    let frame = make_gradient_frame(320, 240);

    // Encode
    encoder.configure(&make_test_config(320, 240)).unwrap();
    encoder.push_frame(&frame).unwrap();
    let mut encoded_packets = Vec::new();
    while let Some(pkt) = encoder.pull_packet().unwrap() {
        encoded_packets.push(pkt);
    }
    encoder.flush().unwrap();
    while let Some(pkt) = encoder.pull_packet().unwrap() {
        encoded_packets.push(pkt);
    }

    assert!(!encoded_packets.is_empty(), "encoder should produce packets");

    // Decode
    for pkt in &encoded_packets {
        decoder.push_packet(&pkt.data).unwrap();
    }
    decoder.flush().unwrap();

    let mut decoded_frames = Vec::new();
    while let Some(frame) = decoder.pull_frame().unwrap() {
        decoded_frames.push(frame);
    }

    assert!(!decoded_frames.is_empty(), "decoder should produce frames");

    let decoded = &decoded_frames[0];
    assert_eq!(decoded.width(), 320);
    assert_eq!(decoded.height(), 240);
}

#[test]
fn roundtrip_psnr_above_35db() {
    let factory = CodecFactory::new();
    let cfg = make_test_config(320, 240);
    let mut encoder = factory.create_encoder(cfg.clone(), None).unwrap();
    encoder.configure(&cfg).unwrap();
    let mut decoder = factory.create_decoder(DecoderConfig { codec: CodecId::H264 }, None).unwrap();
    decoder.configure(&DecoderConfig { codec: CodecId::H264 }).unwrap();
    let frame = make_gradient_frame(320, 240);

    // Collect original Y plane for comparison
    let original_y = frame.plane_data(0).unwrap().to_vec();

    encoder.push_frame(&frame).unwrap();
    let mut packets = Vec::new();
    while let Some(pkt) = encoder.pull_packet().unwrap() { packets.push(pkt); }
    encoder.flush().unwrap();
    while let Some(pkt) = encoder.pull_packet().unwrap() { packets.push(pkt); }

    for pkt in &packets { decoder.push_packet(&pkt.data).unwrap(); }
    decoder.flush().unwrap();

    let mut frames = Vec::new();
    while let Some(f) = decoder.pull_frame().unwrap() { frames.push(f); }

    assert!(!frames.is_empty());
    let decoded_y = frames[0].plane_data(0).unwrap();
    let psnr = compute_psnr(&original_y, decoded_y);
    assert!(psnr > 35.0, "PSNR {psnr:.1} dB should exceed 35 dB");
}
