# Build stage
FROM rust:1.91-slim AS builder

RUN apt-get update && \
    apt-get install -y \
        pkg-config \
        libssl-dev \
        libopencv-dev \
        clang \
        libclang-dev \
        cmake \
        libavcodec-dev \
        libavformat-dev \
        libavutil-dev \
        libavfilter-dev \
        libavdevice-dev \
        libswscale-dev \
        libswresample-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .
RUN cargo build --release --manifest-path backend/Cargo.toml

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y \
        ca-certificates \
        libssl3 \
        libgomp1 && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 app && mkdir -p /app/logs /app/data

WORKDIR /app

COPY --from=builder /app/target/release/roomrtc-server .
COPY backend/identity.pfx .
COPY --from=builder /usr/lib/x86_64-linux-gnu/libopencv*.so.* /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libav*.so.* /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libsw*.so.* /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libpostproc*.so.* /usr/lib/x86_64-linux-gnu/

RUN ldconfig && chown -R app:app /app

USER app

EXPOSE 8080 8081

ENV RUST_LOG=info \
    PORT=8080 \
    TCP_PORT=8081

CMD ["./roomrtc-server"]