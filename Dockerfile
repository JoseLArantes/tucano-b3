# Multi-stage build for tucano-b3
FROM rust:1-bookworm AS builder

WORKDIR /build

# Pre-build dependencies for caching
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/api src/cache src/config src/locale src/middleware src/models src/server src/services && \
    echo 'pub mod api; pub mod cache; pub mod config; pub mod error; pub mod locale; pub mod middleware; pub mod models; pub mod server; pub mod services;' > src/lib.rs && \
    echo 'pub mod pt_br;' > src/locale/mod.rs && \
    echo '// pt_br' > src/locale/pt_br.rs && \
    echo 'pub mod health; pub mod info; pub mod cotahist; pub mod ibov; use axum::Router; pub fn routes() -> Router { Router::new() }' > src/api/mod.rs && \
    echo 'pub struct AppState;' > src/api/health.rs && \
    echo '// info' > src/api/info.rs && \
    echo '// cotahist' > src/api/cotahist.rs && \
    echo '// ibov' > src/api/ibov.rs && \
    echo '// cache' > src/cache/mod.rs && \
    echo '// config' > src/config/mod.rs && \
    echo '// middleware' > src/middleware/mod.rs && \
    echo '// models' > src/models/mod.rs && \
    echo '// server' > src/server/mod.rs && \
    echo '// services' > src/services/mod.rs && \
    echo 'pub enum AppError {}' > src/error.rs && \
    echo 'fn main() {}' > src/main.rs && \
    cargo build --release || true && \
    rm -rf src

# Copy real source code
COPY src/ ./src/

# Rebuild application code
RUN touch src/main.rs src/lib.rs && cargo build --release

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
