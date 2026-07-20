# OMSPBase Pitfalls & Gotchas

## PIT-01: macOS -ObjC linker flag (2026-07-20)

**症状**: `cargo run --example webrtc_loopback_egui --features backend-webrtc-sys` 编译成功但运行崩溃:
```
NSInvalidArgumentException: -[__NSCFConstantString webrtc:: capitalizationStyle]: unrecognized selector sent to instance
```

**根因**: libwebrtc 内部使用 Objective-C categories (NSString+StdString)，macOS 链接器默认会 dead-strip 未被显式引用的 category 方法。`cxx` crate 的 ObjC++ bridge 同样依赖 category 方法。

**解法**: `.cargo/config.toml`:
```toml
[target.x86_64-apple-darwin]
rustflags = ["-C", "link-args=-ObjC -Wl,-no_compact_unwind"]

[target.aarch64-apple-darwin]
rustflags = ["-C", "link-args=-ObjC -Wl,-no_compact_unwind"]
```

`-ObjC` 强制链接器保留所有 ObjC categories，`-no_compact_unwind` 修复 libwebrtc 中 zero-size C++ exception frames 的兼容性问题。

**来源**: webrtc-kit 的 `.cargo/config.toml`。

## PIT-02: webrtc-sys build hangs on macOS without explicit target (2026-07-20)

**症状**: `cargo check --features backend-webrtc-sys` 在 webrtc-sys crate resolution 阶段挂起/超时。

**根因**: webrtc-sys build.rs 触发 libwebrtc 预编译二进制下载 (~200MB)，首次下载耗时较长。在某些网络环境下超时。

**解法**: 
1. 确保网络畅通，首次构建容忍 5-10 分钟
2. 考虑为 CI 添加 `--target` 显式指定
3. 使用 stub backend (`cargo check` 无 features) 快速迭代

## PIT-03: cxx::SharedPtr borrow checker constraints (2026-07-20)

**症状**: webrtc-sys 类型为 `cxx::SharedPtr<T>`，不能跨线程自由传递，需要 `impl_thread_safety!` 宏标记 Send+Sync。

**解法**: webrtc-sys 已通过 `impl_thread_safety!` 标记 PeerConnection/PeerConnectionFactory/DataChannel/SessionDescription 为 Send+Sync。callback-based API 的 ctx 使用 `Box<PeerContext(Box<dyn Any+Send>)>` 传递状态跨 FFI 边界。

## PIT-04: webrtc-rs + webrtc-sys mutual exclusion must be compile_error! (2026-07-20)

**症状**: 同时启用 `backend-webrtc-rs` 和 `backend-webrtc-sys` features 导致 type alias 冲突（两个 backend 都声明 `ActivePc`）。

**解法**: `backend/mod.rs` 中:
```rust
#[cfg(all(feature = "backend-webrtc-rs", feature = "backend-webrtc-sys"))]
compile_error!("Only one backend can be enabled at a time.");
```

## PIT-05: egui example compilation requires full dependency tree (2026-07-20)

**症状**: `backend-webrtc-sys` feature 下 egui 示例需要 eframe/egui 完整编译（40+ crates, ~10 分钟）。

**解法**: 接受首次编译时间。后续增量编译仅需 1-2 分钟。
