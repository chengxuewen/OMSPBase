mod common;

use omspbase_media::backends::NativeTransform;
use omspbase_media::base::buffer::{I420Buffer, VideoBuffer};
use omspbase_media::base::rotation::VideoRotation;
use omspbase_media::transform::VideoTransform;

/// Build an `I420BufferRef` from an I420 buffer for backend calls.
fn i420_ref(buf: &I420Buffer) -> omspbase_media::base::buffer::I420BufferRef<'_> {
    buf.as_i420_ref().unwrap()
}

// ── scale ────────────────────────────────────────────────

#[test]
fn scale_down_by_two() {
    let src_frame = common::create_test_frame(64, 64);
    let src = src_frame.buffer.to_i420().unwrap();
    let dst = I420Buffer::new(32, 32);
    let sr = i420_ref(&src);
    let dr = i420_ref(&dst);
    NativeTransform::scale(sr, 64, 64, dr, 32, 32).unwrap();

    assert_eq!(dst.width(), 32);
    assert_eq!(dst.height(), 32);
    // ponytail: verify at least one bright pixel (content was copied from center square)
    assert!(
        dst.data_y.iter().any(|&p| p > 16),
        "Expected some bright pixels after downscale"
    );
}

#[test]
fn scale_up_by_two() {
    let src_frame = common::create_test_frame(64, 64);
    let src = src_frame.buffer.to_i420().unwrap();
    let dst = I420Buffer::new(128, 128);
    let sr = i420_ref(&src);
    let dr = i420_ref(&dst);
    NativeTransform::scale(sr, 64, 64, dr, 128, 128).unwrap();

    assert_eq!(dst.width(), 128);
    assert_eq!(dst.height(), 128);
    assert!(
        dst.data_y.iter().any(|&p| p > 16),
        "Expected some bright pixels after upscale"
    );
}

#[test]
fn scale_identity_is_copy() {
    let src_frame = common::create_test_frame(64, 64);
    let src = src_frame.buffer.to_i420().unwrap();
    let dst = I420Buffer::new(64, 64);
    let sr = i420_ref(&src);
    let dr = i420_ref(&dst);
    NativeTransform::scale(sr, 64, 64, dr, 64, 64).unwrap();

    assert_eq!(dst.width(), 64);
    assert_eq!(dst.height(), 64);
    // Same-size scale should be a pixel-exact copy
    assert_eq!(dst.data_y, src.data_y, "Identity scale should preserve Y plane");
    assert_eq!(dst.data_u, src.data_u, "Identity scale should preserve U plane");
    assert_eq!(dst.data_v, src.data_v, "Identity scale should preserve V plane");
}

// ── mirror ────────────────────────────────────────────────

#[test]
fn mirror_vertical_top_to_bottom() {
    // Create 64×64 frame: top 8 rows = 200, bottom 8 rows = 50
    let mut src = I420Buffer::new(64, 64);
    let stride_y = src.stride_y as usize;
    for y in 0..8usize {
        let off = y * stride_y;
        src.data_y[off..off + 64].fill(200);
    }
    for y in (64 - 8)..64usize {
        let off = y * stride_y;
        src.data_y[off..off + 64].fill(50);
    }
    src.data_u.fill(128);
    src.data_v.fill(128);

    let dst = I420Buffer::new(64, 64);
    let sr = i420_ref(&src);
    let dr = i420_ref(&dst);
    NativeTransform::mirror(sr, 64, 64, dr).unwrap();

    // After vertical mirror: old top rows (200) should now be at bottom
    let top_row = &dst.data_y[0..64];
    let bottom_row = &dst.data_y[(63 * stride_y)..(63 * stride_y + 64)];
    assert!(
        top_row.iter().all(|&p| p == 50),
        "Top row should now contain old-bottom values (50)"
    );
    assert!(
        bottom_row.iter().all(|&p| p == 200),
        "Bottom row should now contain old-top values (200)"
    );
}

#[test]
fn mirror_is_idempotent() {
    let mut src = I420Buffer::new(8, 8);
    let stride_y = src.stride_y as usize;
    for i in 0..8u8 {
        src.data_y[i as usize * stride_y + i as usize] = 200;
    }

    // First mirror
    let tmp = I420Buffer::new(8, 8);
    let sr = i420_ref(&src);
    let tr = i420_ref(&tmp);
    NativeTransform::mirror(sr, 8, 8, tr).unwrap();

    // Second mirror — should restore original
    let dst = I420Buffer::new(8, 8);
    let tr = i420_ref(&tmp);
    let dr = i420_ref(&dst);
    NativeTransform::mirror(tr, 8, 8, dr).unwrap();

    assert_eq!(dst.data_y, src.data_y, "Double mirror should restore original");
}

// ── crop ──────────────────────────────────────────────────

#[test]
fn crop_center_32x32() {
    let src_frame = common::create_test_frame(64, 64);
    let src = src_frame.buffer.to_i420().unwrap();
    let dst = I420Buffer::new(32, 32);
    let sr = i420_ref(&src);
    let dr = i420_ref(&dst);
    // Center 32×32: offset (16, 16)
    NativeTransform::crop(sr, 16, 16, 32, 32, dr).unwrap();

    assert_eq!(dst.width(), 32);
    assert_eq!(dst.height(), 32);
    // The bright 16×16 center square in create_test_frame spans [24,40) in src coords.
    // Offset (16,16) means crop window is [16,48). Center is at [32,48) in src,
    // which is [16,32) in crop coords. Verify pixel at (16,16) in dst is bright.
    let stride_y = dst.stride_y as usize;
    let center_pixel = dst.data_y[16 * stride_y + 16];
    assert!(center_pixel > 16, "Center pixel of crop should contain bright content");
}

#[test]
fn crop_top_left_corner() {
    let mut src = I420Buffer::new(8, 8);
    let stride_y = src.stride_y as usize;
    // Fill with position-dependent values: row*stride + col
    for y in 0..8usize {
        for x in 0..8usize {
            src.data_y[y * stride_y + x] = (y * 8 + x) as u8;
        }
    }

    let dst = I420Buffer::new(4, 4);
    let sr = i420_ref(&src);
    let dr = i420_ref(&dst);
    NativeTransform::crop(sr, 0, 0, 4, 4, dr).unwrap();

    // Top-left 4×4 of an 8×8 with row-major values
    assert_eq!(dst.data_y[0], 0);
    assert_eq!(dst.data_y[3], 3);
    assert_eq!(dst.data_y[3 * 4], 24); // row 3, col 0 → 3*8+0 = 24
    assert_eq!(dst.data_y[3 * 4 + 3], 27); // row 3, col 3 → 3*8+3 = 27
}

#[test]
fn crop_bottom_right_corner() {
    let mut src = I420Buffer::new(8, 8);
    let stride_y = src.stride_y as usize;
    for y in 0..8usize {
        for x in 0..8usize {
            src.data_y[y * stride_y + x] = (y * 8 + x) as u8;
        }
    }

    let dst = I420Buffer::new(4, 4);
    let sr = i420_ref(&src);
    let dr = i420_ref(&dst);
    NativeTransform::crop(sr, 4, 4, 4, 4, dr).unwrap();

    // Bottom-right 4×4 of an 8×8 with row-major values
    // row 4, col 4 → 4*8+4 = 36
    assert_eq!(dst.data_y[0], 36);
    // row 7, col 7 → 7*8+7 = 63
    assert_eq!(dst.data_y[3 * 4 + 3], 63);
}

// ── rotate ────────────────────────────────────────────────

#[test]
fn rotate_90_swaps_pixel_positions() {
    // Create a 4×2 frame: row 0 = [0,1,2,3], row 1 = [4,5,6,7]
    let mut src = I420Buffer::new(4, 2);
    for i in 0..8u8 {
        src.data_y[i as usize] = i;
    }
    src.data_u.fill(128);
    src.data_v.fill(128);

    // After 90° CW: 4×2 → 2×4 (width and height swap)
    let dst = I420Buffer::new(2, 4);
    let sr = i420_ref(&src);
    let dr = i420_ref(&dst);
    NativeTransform::rotate(sr, 4, 2, VideoRotation::Rotation90, dr).unwrap();

    assert_eq!(dst.width(), 2);
    assert_eq!(dst.height(), 4);
    // src[0,0]=0 rotates to dx=0, dy=4-1-0=3 → dst[3][0] = dst[6]
    assert_eq!(dst.data_y[6], 0, "Top-left source pixel should map to bottom-left after 90 CW");
    // src[0,3]=3 rotates to dx=0, dy=4-1-3=0 → dst[0][0] = dst[0]
    assert_eq!(dst.data_y[0], 3, "Top-right source pixel should map to top-left after 90 CW");
}

#[test]
fn rotate_180_inverts_pixel_positions() {
    let mut src = I420Buffer::new(64, 64);
    let stride_y = src.stride_y as usize;
    // Mark four corners with distinct values
    src.data_y[0] = 10; // top-left
    src.data_y[63] = 20; // top-right
    src.data_y[63 * stride_y] = 30; // bottom-left
    src.data_y[63 * stride_y + 63] = 40; // bottom-right

    let dst = I420Buffer::new(64, 64);
    let sr = i420_ref(&src);
    let dr = i420_ref(&dst);
    NativeTransform::rotate(sr, 64, 64, VideoRotation::Rotation180, dr).unwrap();

    assert_eq!(dst.width(), 64);
    assert_eq!(dst.height(), 64);
    // 180° rotation: each corner maps to opposite corner
    assert_eq!(dst.data_y[0], 40, "Top-left should be old bottom-right");
    assert_eq!(dst.data_y[63], 30, "Top-right should be old bottom-left");
    assert_eq!(dst.data_y[63 * stride_y], 20, "Bottom-left should be old top-right");
    assert_eq!(dst.data_y[63 * stride_y + 63], 10, "Bottom-right should be old top-left");
}

#[test]
fn rotate_270_restores_after_90() {
    // Create a 4×2 frame
    let mut src = I420Buffer::new(4, 2);
    for i in 0..8u8 {
        src.data_y[i as usize] = i;
    }
    src.data_u.fill(128);
    src.data_v.fill(128);

    // Rotate 90 → intermediate
    let mid = I420Buffer::new(2, 4);
    let sr = i420_ref(&src);
    let mr = i420_ref(&mid);
    NativeTransform::rotate(sr, 4, 2, VideoRotation::Rotation90, mr).unwrap();

    // Rotate 270 on intermediate (same input dimensions as intermediate: 2×4)
    let dst = I420Buffer::new(4, 2);
    let mr = i420_ref(&mid);
    let dr = i420_ref(&dst);
    NativeTransform::rotate(mr, 2, 4, VideoRotation::Rotation270, dr).unwrap();

    // 270° CW (which is 90° CCW) on the 90°-rotated result should restore the original
    assert_eq!(dst.data_y, src.data_y, "90 + 270 should restore original pixel layout");
}

#[test]
fn rotate_0_is_identity() {
    let mut src = I420Buffer::new(8, 8);
    let stride_y = src.stride_y as usize;
    for y in 0..8usize {
        for x in 0..8usize {
            src.data_y[y * stride_y + x] = (y * 8 + x) as u8;
        }
    }

    let dst = I420Buffer::new(8, 8);
    let sr = i420_ref(&src);
    let dr = i420_ref(&dst);
    NativeTransform::rotate(sr, 8, 8, VideoRotation::Rotation0, dr).unwrap();

    assert_eq!(dst.data_y, src.data_y, "Rotation0 should be identity");
}
