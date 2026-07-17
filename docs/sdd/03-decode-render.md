# SDD 03: Decode + Render

## 1. 概述

视频解码 + 渲染模块。从 WebRTC 接收编码帧，解码为 I420，转换为 GPU 纹理输出到屏幕。

**决策引用**: D46 (VideoDecode trait), D47 (VideoRender trait), D75 (I420 标准)

## 2. 接口定义

```rust
pub trait VideoDecoder: Send {
    fn supported_codecs(&self) -> Vec<Codec>;
    fn decode(&mut self, encoded: EncodedFragment) -> Result<RawFrame>;
    fn request_keyframe(&mut self) -> Result<()>;
    fn reset(&mut self) -> Result<()>;
    fn caps(&self) -> DecoderCaps;
}

pub trait VideoRenderer: Send {
    fn set_surface(&mut self, target: &RenderTarget) -> Result<()>;
    fn render(&mut self, frame: RawFrame) -> Result<()>;
    fn caps(&self) -> RenderCaps;
}

bitflags! {
    pub struct DecoderCaps: u32 {
        const H264      = 1 << 0;
        const HEVC      = 1 << 1;
        const VP8       = 1 << 2;
        const VP9       = 1 << 3;
        const GPU_TEXTURE = 1 << 4;
        const ADAPTIVE  = 1 << 5;  // 丢包恢复
    }
}

bitflags! {
    pub struct RenderCaps: u32 {
        const CPU_FALLBACK = 1 << 0;
        const GPU_DIRECT   = 1 << 1;
    }
}
```

### 帧结构

```rust
pub struct RawFrame {
    pub data: FrameData,         // I420 data 或 GPU texture handle
    pub width: u32,
    pub height: u32,
    pub pts: u64,
}

pub enum FrameData {
    I420(Vec<u8>),               // Phase 1 CPU 路径
    Texture(TextureHandle),      // Phase 2 GPU 路径
}
```

## 3. 管线流程

```
WebRTC on_message
    │ EncodedFragment (H.264 Annex-B)
    ▼
VideoDecoder::decode (GStreamer decodebin)
    │ RawFrame (I420)
    ▼
I420 → YUV→RGB 转换 (CPU Phase 1 / shader Phase 2)
    │ RGBA buffer
    ▼
VideoRenderer::render → 输出到窗口/Canvas
```

Phase 1 CPU 回读路径: `appsink → CPU buffer → Canvas 2D`

## 4. 后端策略

| 后端 | 场景 | 备注 |
|------|------|------|
| GStreamer decodebin | Phase 1 通用 | appsrc → h264parse → decodebin → videoconvert → appsink |
| FFmpeg libavcodec | remote SDK 静态链接 | 仅 backend-str0m 场景 |
| WebCodecs | 浏览器 | 跳过 codec crate |

## 5. 错误处理

| 条件 | 分类 | 错误码 | 恢复 |
|------|------|--------|------|
| 解码器初始化失败 | Fatal | 5001 | 退出并提示修改配置 |
| 解码帧格式无效 | Recoverable | 5002 | 丢弃该帧, request_keyframe |
| 渲染 surface 无效 | Recoverable | 5002 | 跳过渲染, 等待新 surface |

## 6. 测试计划

| 测试 | 类型 | 描述 |
|------|------|------|
| decode_h264_baseline | 集成 | 输入 H.264 Annex-B, 输出 I420 帧完整 |
| decode_latency_budget | 集成 | 单帧解码延迟 <50ms (720p) |
| frame_integrity | 集成 | 解码后帧 YUV 各平面大小正确, PSNR > 40dB |
| keyframe_request | 单元 | request_keyframe 后收到 IDR 帧 |
| decoder_reset | 单元 | reset 后解码器状态清除, 可重新解码 |
| render_rgba_output | 集成 | render 后输出 RGBA buffer 尺寸正确 |
| phase1_cpu_path | 集成 | CPU fallback 路径帧率 > 20fps |
| format_conversion | 单元 | I420 → RGBA 转换色值正确 |