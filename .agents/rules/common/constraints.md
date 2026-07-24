# Development Constraints

## Git Commit Rules

### Cargo.lock Must Be Committed
**ALWAYS** commit `Cargo.lock` along with dependency changes. This file tracks exact dependency versions and must be in sync with `Cargo.toml`.

Common mistake: Forgetting to `git add Cargo.lock` after `Cargo.toml` changes. This causes build failures for other developers.

**Checklist before committing:**
- [ ] `Cargo.toml` changes committed
- [ ] `Cargo.lock` changes committed (if dependencies changed)
- [ ] `git status` shows clean working tree

## Platform Constraints

### macOS Development — Host/Client Native, Server Docker
- **Host (`omspbase-host`) and Client (`omspbase-client`)**: Develop and run natively on macOS. These crates do not depend on mediasoup.
- **Server (`omspbase-server`)**: Compile and run via Docker when `sfu-mediasoup` feature is enabled. The server binary and `cargo check` work natively on macOS, but mediasoup integration requires a Linux container.
- Use `docker compose up -d` for the server dev container. See `docs/modules/development/docker-workflow.md`.

### mediasoup Only Builds on Linux x86_64
- mediasoup's C++ Worker (compiled via meson/ninja) is a **Linux x86_64-only** native binary. It does not build on macOS ARM64 or Windows.
- **macOS workflow**: `cargo check --features sfu-mediasoup` works (checks Rust bindings), but `cargo build` or `cargo test` with `sfu-mediasoup` fails. Full compilation and testing require a Linux environment.
- **Docker workflow**: The `dev` container image (rust:stable-bookworm + meson) provides the full mediasoup build environment.
- **CI**: The `test-mediasoup` job runs on `ubuntu-latest` only (see `.github/workflows/ci.yml`).

### CI: test-mediasoup Runs on ubuntu-latest Only
- `.github/workflows/ci.yml` defines `test-mediasoup` with `runs-on: ubuntu-latest`. It installs meson, ninja-build, libuv1-dev, and libssl-dev before running `cargo test -p omspbase-server --features sfu-mediasoup`.
- The `check` and `test` jobs do run on both `ubuntu-latest` and `macos-latest` (for workspace-level validation without mediasoup features).

## Docker Constraints

### Docker Desktop Volume Mount Performance
- Docker Desktop on macOS uses **osxfs (legacy) or virtiofs (newer)** for bind mounts. Both are **3-5x slower** than native Linux filesystem access.
- The `cargo-cache` named volume in `docker-compose.yml` mitigates this for dependency downloads, but **workspace source code bind mounts are still slow**.
- **Mitigation**: Prefer `docker compose exec` for running cargo commands inside the container rather than relying on host-side tooling. Avoid running `cargo build` from a host-mounted volume for large builds — use the container's internal filesystem or a dedicated volume.
- First-time `cargo build` with `sfu-mediasoup` inside Docker can take **15-30 minutes** (vs. 3-5 minutes for native Linux).

## Network Constraints

### UDP Port Range for mediasoup
- mediasoup Worker RTP/RTCP uses **UDP ports 40000-40100** by default.
- Port mapping in `docker-compose.yml`:
  ```
  40000-40100:40000-40100/udp
  ```
- When deploying outside Docker, ensure the host firewall allows this UDP range. For production, narrow the range (e.g., `rtc_ports_range: (40000, 40100)`) — fewer ports reduce firewall surface area.

### ICE/STUN Required for Local P2P
- WebRTC (even on localhost) requires **ICE negotiation** with STUN to discover candidate pairs. Without a STUN server, localhost WebRTC connections fail because no candidate pairs are formed.
- **Development setup**: Run a STUN server (e.g., `coturn` or `stuntman`) locally, or configure the WebRTC transport to use a host-loopback ICE candidate.
- **Common gotcha**: Host and Client on the same machine assume localhost WebRTC "just works" — it does not. ICE must be configured explicitly even for loopback connections.
- mediasoup's `WebRtcTransport` uses ICE-Lite (server-side) by default, which reduces the ICE handshake to one round trip but still requires the client to send a STUN binding request.

## macOS-Specific Gotchas

| Gotcha | Detail |
|--------|--------|
| mediasoup build fails | C++ Worker requires Linux + meson. Use Docker on macOS. |
| `cargo test --features sfu-mediasoup` fails on macOS | Runs fine on ubuntu-latest CI. macOS can only `check`. |
| Docker Desktop slow | Volume mounts 3-5x slower than native. Use cargo-cache volume. |
| First Docker build takes 15-30 min | mediasoup C++ Worker + Rust deps from scratch. |
| localhost WebRTC fails without STUN | ICE needs explicit candidates — even for loopback. |
| Cargo.lock drift | Always commit `Cargo.lock` alongside `Cargo.toml` changes. |