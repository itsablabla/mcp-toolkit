# Multi-stage Dockerfile for mcp-toolkit
# Stage 1: Build the Rust binary
FROM rust:1.94-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

# Build release binary
RUN cargo build --release --bin mcp-toolkit

# Stage 2: Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && npm install -g npx \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/mcp-toolkit /usr/local/bin/mcp-toolkit

# Create config directory
RUN mkdir -p /root/.mcp-toolkit

# Default config volume
VOLUME ["/root/.mcp-toolkit"]

# Expose MCP server port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

ENTRYPOINT ["mcp-toolkit"]
CMD ["serve", "--bind", "0.0.0.0:3000"]
