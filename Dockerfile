# Multi-stage build for Tucano B3 (Rust Edition)
FROM rust:1-bookworm AS builder

WORKDIR /usr/src/tucano-b3

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Pre-build dependencies with a dummy entrypoint to cache build layers
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs
RUN cargo build --release
RUN rm -rf src

# Copy application source and migrations
COPY migrations ./migrations
COPY src ./src

# Rebuild only the application code
RUN touch src/main.rs src/lib.rs
RUN cargo build --release --bin tucano-b3

# Runtime Stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy compiled binary from builder
COPY --from=builder /usr/src/tucano-b3/target/release/tucano-b3 /usr/local/bin/tucano-b3

# Create directories for raw files
RUN mkdir -p /data/raw /data/processed

ENV B3_RAW_DATA_DIR=/data/raw
ENV LOG_LEVEL=INFO

EXPOSE 8000

CMD ["tucano-b3"]
