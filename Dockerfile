# Build stage
FROM rust:bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --shell /bin/bash tucano

# Copy binary and config
COPY --from=builder /build/target/release/tucano-b3 /usr/local/bin/tucano-b3
COPY config.toml /etc/tucano/config.toml

# Create data directory
RUN mkdir -p /data && chown tucano:tucano /data

USER tucano

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
    CMD curl -f http://localhost:8080/api/v1/health || exit 1

ENTRYPOINT ["tucano-b3"]
CMD ["--config", "/etc/tucano/config.toml"]

