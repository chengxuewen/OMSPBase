# SDD 01: CameraCapture

## 1. 概述

摄像头采集模块。将平台摄像头裸流转换为统一 I420 帧，输入到 PipelineEngine。

**决策引用**: D64 (CameraCapture trait), D75 (I420 裸流标准)

## 2. 接口定义

```rust
/// 摄像头源类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CameraSource {
    V4L2(String),         // /dev/videoN (Linux)
    AVFoundation(String), // 设备 UID (macOS)
    DirectShow(String),   // 设备名 (Windows)
    RTSP(String),         // rtsp:// 地址
}

bitflags! {
    pub struct CameraCaps: u32 {
        const I420       = 1 << 0;  // I420 原生输出
        const NV12       = 1 << 1;  // NV12 原生输出
        const MJPG       = 1 << 2;  // MJPEG 压缩
        const AUTO_FOCUS = 1 << 3;
        const HOTPLUG    = 1 << 4;  // 热插拔支持
    }
}

#[async_trait]
pub trait CameraCapture: Send + Sync {
    fn name(&self) -> &str;
    fn caps(&self) -> CameraCaps;
    fn list_devices(&self) -> Result<Vec<CameraDevice>>;
    async fn open(&mut self, source: &CameraSource) -> Result<()>;
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn poll_frame(&mut self) -> Result<Option<I420Frame>>;
    fn resolution(&self) -> (u32, u32);
    fn fps(&self) -> u32;
}
```

### 输出帧结构

```rust
pub struct I420Frame {
    pub y: Vec<u8>,         // Y 平面
    pub u: Vec<u8>,         // U 平面 (宽高各半)
    pub v: Vec<u8>,         // V 平面 (宽高各半)
    pub width: u32,
    pub height: u32,
    pub pts: u64,           // 采集时间戳 (μs)
    pub source: CameraSource,
}
```

## 3. 平台实现

| 平台 | 后端 | 采集元 |
|------|------|--------|
| Linux | GStreamer v4l2src | /dev/videoN, autovideosrc |
| macOS | GStreamer avfvideosrc | AVFoundation 设备 |
| Windows | GStreamer dshowvideosrc | DirectShow 设备 |

## 4. 错误处理 (D-ERR-01)

| 条件 | 分类 | 错误码 | 恢复 |
|------|------|--------|------|
| 设备不存在 | Fatal | 3001 | 重试 3 次后退出 |
| 帧丢失 | Recoverable | 3002 | 跳过该帧，继续采集 |
| 分辨率不支持 | Recoverable | 3003 | fallback 到 720p |
| 热插拔断开 | Recoverable | 3002 | 等待 1s 重建 pipeline |

## 5. Pipeline 集成

```
CameraSource → GStreamer pipeline → appsink poll
     │              │                      │
     │              ▼                      ▼
     └── videoconvert ──▶ I420Frame ──▶ PipelineEngine
```

通过 `poll_frame` 轮询模式集成，PipelineEngine 驱动采集循环。

## 6. 测试计划

| 测试 | 类型 | 描述 |
|------|------|------|
| frame_format_validation | 单元 | 输出帧为 I420, width/height 匹配, YUV 平面大小正确 |
| resolution_change | 单元 | 切换分辨率后帧尺寸正确更新 |
| device_list | 单元 | list_devices 返回非空列表 |
| open_close | 集成 | open → start → stop → close 生命周期完整 |
| device_hotplug | 集成 | 设备断开后重建 pipeline (mock udev) |
| fps_stability | 集成 | 30fps 设定下实测 ≥28fps |
| rtsp_source | 集成 | RTSP 源连接和帧接收 |
| error_recovery_fatal | 单元 | 3001 错误 3 次后触发退出流程 |