# Multi-stage build for tucano-b3
FROM rust:1-bookworm AS builder

WORKDIR /build

# Pre-build dependencies for caching
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src benches && \
    echo 'fn main() {}' > benches/data_layer.rs && \
    touch src/lib.rs && \
    echo 'fn main() {}' > src/main.rs && \
    cargo build --release --bin tucano-b3 && \
    rm -rf src benches

# Copy real source code
COPY src/ ./src/
COPY benches/ ./benches/

# Rebuild application binary
RUN touch src/main.rs src/lib.rs && cargo build --release --bin tucano-b3

# Runtime stage
FROM debian:bookworm-slim

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --shell /bin/bash tucano

WORKDIR /app

# Copy binary and configuration
COPY --from=builder /build/target/release/tucano-b3 /usr/local/bin/tucano-b3
COPY config.toml /etc/tucano/config.toml

# Create data directory with appropriate permissions
RUN mkdir -p /data && chown -R tucano:tucano /data /etc/tucano

USER tucano

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
    CMD curl -f http://localhost:8080/api/v1/health || exit 1

ENTRYPOINT ["tucano-b3"]
CMD ["--config", "/etc/tucano/config.toml"]
