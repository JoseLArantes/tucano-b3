# ============================================================
# Build stage
# ============================================================
FROM rust:1-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    clang \
    lld \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTFLAGS="-C link-arg=-fuse-ld=lld"

WORKDIR /build

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY benches/ ./benches/
RUN mkdir -p src && \
    touch src/lib.rs && \
    echo 'fn main() {}' > src/main.rs && \
    cargo build --release --bin tucano-b3 && \
    rm -rf src

# Build actual application
COPY src/ ./src/
RUN touch src/main.rs src/lib.rs && \
    cargo build --release --bin tucano-b3

# ============================================================
# Runtime stage
# ============================================================
FROM debian:bookworm-slim AS runtime

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user
RUN groupadd --gid 1000 tucano \
    && useradd --uid 1000 --gid 1000 --create-home --shell /bin/bash tucano

# Copy binary
COPY --from=builder /build/target/release/tucano-b3 /usr/local/bin/tucano-b3

# Copy default config and api keys
COPY config.toml /etc/tucano/config.toml
COPY api_keys.toml /etc/tucano/api_keys.toml

# Create data directory
RUN mkdir -p /data/processed /data/ledger \
    && chown -R tucano:tucano /data /etc/tucano

USER tucano

EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -sf http://localhost:8080/api/v1/health || exit 1

# Labels
LABEL org.opencontainers.image.title="tucano-b3" \
      org.opencontainers.image.description="API REST de alta performance para dados de mercado da B3" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0" \
      org.opencontainers.image.vendor="tucano-b3"

ENTRYPOINT ["tucano-b3"]
CMD ["--config", "/etc/tucano/config.toml"]
