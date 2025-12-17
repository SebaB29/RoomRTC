FROM rust:1.91-slim AS builder

# Install system dependencies for OpenCV and GUI
RUN apt-get update && apt-get install -y \
    pkg-config \
    libopencv-dev \
    clang \
    libclang-dev \
    libssl-dev \
    libgtk-3-dev \
    libx11-dev \
    libxext-dev \
    libxrender-dev \
    libxcb1-dev \
    libxcb-render0-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libxkbcommon-dev \
    libwayland-dev \
    libegl1-mesa-dev \
    libgles2-mesa-dev \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libavfilter-dev \
    libavdevice-dev \
    libswscale-dev \
    libswresample-dev \
    libxrandr-dev \
    libxcursor-dev \
    libxi-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files
COPY frontend/Cargo.toml ./Cargo.toml
COPY frontend/src ./src
COPY frontend/app.conf ./

# Copy shared dependencies (mantener estructura relativa)
COPY shared ../shared
COPY webrtc ../webrtc

# Build the application in release mode
RUN cargo build --release --bin frontend

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies for OpenCV and GUI
RUN apt-get update && apt-get install -y \
    libopencv-core4.6 \
    libopencv-highgui4.6 \
    libopencv-videoio4.6 \
    libopencv-imgproc4.6 \
    libgtk-3-0 \
    libx11-6 \
    libxext6 \
    libxrender1 \
    libxcb1 \
    libxcb-render0 \
    libxcb-shape0 \
    libxcb-xfixes0 \
    libxkbcommon0 \
    libwayland-client0 \
    libwayland-cursor0 \
    libwayland-egl1 \
    libegl1 \
    libgles2 \
    ca-certificates \
    ffmpeg \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libavfilter-dev \
    libswscale-dev \
    libswresample-dev \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 rustuser && \
    mkdir -p /home/rustuser/.config && \
    chown -R rustuser:rustuser /home/rustuser

# Copy binary and config from builder
COPY --from=builder /app/target/release/frontend /usr/local/bin/frontend
COPY --from=builder /app/app.conf /home/rustuser/app.conf

# Set user
USER rustuser
WORKDIR /home/rustuser

# Expose display for X11 forwarding (optional)
ENV DISPLAY=:0

# Run the application
CMD ["frontend"]
