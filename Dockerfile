# === 1. Build Stage ===
FROM rust:1.87 AS builder

ARG SERVICE_NAME

WORKDIR /app

# Recreate full workspace to avoid rebuild
COPY Cargo.toml Cargo.lock ./
COPY services/ ./services/

# Build only the service
RUN cargo build --release -p ${SERVICE_NAME}

# === 2. Runtime Stage ===
FROM debian:bookworm-slim

ARG SERVICE_NAME
ENV SERVICE_NAME=${SERVICE_NAME}

# Install necessary system libs (if needed, e.g. for OpenSSL)
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/${SERVICE_NAME} .

ENV RUST_LOG=info

EXPOSE 8080

CMD ["sh", "-c", "exec ./${SERVICE_NAME}"]
