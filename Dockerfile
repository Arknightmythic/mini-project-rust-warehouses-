# ---- Build stage ----
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release --bin mini-warehouse-wms --bin seed

# ---- Runtime stage ----
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/mini-warehouse-wms /app/mini-warehouse-wms
COPY --from=builder /app/target/release/seed /app/seed
COPY seeders ./seeders

EXPOSE 8080

CMD ["/app/mini-warehouse-wms"]
