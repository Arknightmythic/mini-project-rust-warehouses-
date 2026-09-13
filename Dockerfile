# ---- Build stage ----
FROM rust:1-slim-bookworm AS builder
ARG SERVICE
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Cache mounts are not persisted into the image layer, so everything that must
# survive into the runtime stage has to be copied out inside this same RUN.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo build --release -p ${SERVICE} && \
    mkdir -p /out && \
    cp /app/target/release/${SERVICE} /out/service && \
    if [ -f /app/target/release/${SERVICE}-seed ]; then \
        cp /app/target/release/${SERVICE}-seed /out/seed; \
    fi && \
    if [ -d crates/${SERVICE}/seeders ]; then \
        cp -r crates/${SERVICE}/seeders /out/seeders; \
    fi

# ---- Runtime stage ----
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /out /app

EXPOSE 8080

CMD ["/app/service"]
