FROM rust:1.85 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/kinetic /usr/local/bin/kinetic
COPY Rocket.toml /app/Rocket.toml
COPY templates /app/templates
COPY static /app/static
EXPOSE 8000
ENV ROCKET_ADDRESS=0.0.0.0 ROCKET_PORT=8000 ROCKET_PROFILE=release
CMD ["kinetic"]
