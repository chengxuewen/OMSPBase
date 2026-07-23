# ---- Base: Rust + system deps ----
FROM rust:stable-bookworm AS base
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config cmake ninja-build git \
    libssl-dev libuv1-dev \
    python3 python3-pip \
    && rm -rf /var/lib/apt/lists/*
# Meson for mediasoup C++ Worker
RUN pip3 install --break-system-packages meson

# ---- Dev: full toolchain + source ----
FROM base AS dev
WORKDIR /workspace
COPY . .
RUN cargo fetch
CMD ["bash"]

# ---- Builder: release build ----
FROM base AS builder
WORKDIR /workspace
COPY . .
RUN cargo build --release --bin omspbase-server

# ---- Runtime: minimal image ----
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 libuv1 ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /workspace/target/release/omspbase-server /usr/local/bin/
EXPOSE 8000 40000-40100/udp
ENTRYPOINT ["omspbase-server"]