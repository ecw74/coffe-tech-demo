# === 1. Build Stage ===
FROM rust:1.87 AS builder

WORKDIR /app

# Recreate full workspace to avoid rebuild
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build only the order-service
RUN cargo build --release -p order-service

# === 2. Runtime Stage ===
FROM debian:bookworm-slim

# Install necessary system libs (if needed, e.g. for OpenSSL)
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/order-service .

ENV RUST_LOG=info

EXPOSE 8080

CMD ["./order-service"]
