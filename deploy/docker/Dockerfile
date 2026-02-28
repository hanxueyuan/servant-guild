# ServantGuild Production Dockerfile
# Multi-stage build for optimized production image

# =============================================================================
# Stage 1: Build dependencies and compile
# =============================================================================
FROM rust:1.87-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    protobuf-compiler \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Create workspace
WORKDIR /app

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY wit ./wit
COPY tests ./tests

# Build the application
RUN cargo build --release --locked

# Strip symbols for smaller binary
RUN strip /app/target/release/servant-guild || true

# =============================================================================
# Stage 2: Wasm modules build
# =============================================================================
FROM rust:1.87-bookworm AS wasm-builder

# Install wasm target
RUN rustup target add wasm32-unknown-unknown
RUN rustup component add rust-src

# Install wasm tools
RUN cargo install wasm-opt

WORKDIR /wasm

# Copy servant modules
COPY src/servants ./servants

# Build Wasm modules (example for coordinator)
# In production, this would build each servant module
RUN mkdir -p /wasm/output

# =============================================================================
# Stage 3: Production runtime
# =============================================================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /bin/false servant

# Create directories
WORKDIR /app
RUN mkdir -p /app/config /app/data /app/logs /app/wasm /app/secrets \
    && chown -R servant:servant /app

# Copy binary from builder
COPY --from=builder /app/target/release/servant-guild /app/servant-guild

# Copy Wasm modules (if built)
# COPY --from=wasm-builder /wasm/output/*.wasm /app/wasm/

# Copy default configuration
COPY config/default.toml /app/config/default.toml

# Set environment
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV SERVANT_GUILD_CONFIG=/app/config/config.toml
ENV SERVANT_GUILD_DATA=/app/data
ENV SERVANT_GUILD_LOGS=/app/logs

# Expose ports
# 8080: Main API
# 9090: Metrics
EXPOSE 8080 9090

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Switch to non-root user
USER servant

# Entry point
ENTRYPOINT ["/app/servant-guild"]
CMD ["run", "--config", "/app/config/config.toml"]

# =============================================================================
# Stage 4: Development image
# =============================================================================
FROM runtime AS development

USER root

# Install development tools
RUN apt-get update && apt-get install -y \
    vim \
    less \
    procps \
    htop \
    net-tools \
    && rm -rf /var/lib/apt/lists/*

USER servant

# =============================================================================
# Labels
# =============================================================================
LABEL org.opencontainers.image.title="ServantGuild"
LABEL org.opencontainers.image.description="Autonomous multi-agent system with Wasm-driven servants"
LABEL org.opencontainers.image.version="0.4.0"
LABEL org.opencontainers.image.authors="ServantGuild Team"
LABEL org.opencontainers.image.source="https://github.com/hanxueyuan/servant-guild"
