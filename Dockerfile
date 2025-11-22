# Use bookworm-based Rust image so it matches the runtime
FROM rust:1.91-bookworm AS builder
# or: FROM rust:1.91-bookworm-slim AS builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY .sqlx .sqlx
COPY src ./src
COPY migrations ./migrations

ENV SQLX_OFFLINE=true

RUN cargo build --release --locked

# Runtime stage: same Debian family (bookworm)
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && update-ca-certificates \
    && groupadd -r nebula && useradd -r -g nebula nebula

WORKDIR /app

COPY --from=builder /app/target/release/nebula-backend /usr/local/bin/nebula-backend
COPY --from=builder /app/migrations ./migrations

RUN chown -R nebula:nebula /app
USER nebula

# Your app actually listens on 3838 (BACKEND_ADDR=0.0.0.0:3838),
# so it's nicer to expose that, even though EXPOSE is just metadata.
EXPOSE 3838

ENV RUST_LOG=info

CMD ["nebula-backend"]

