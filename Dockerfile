FROM rust:1-bookworm AS builder
WORKDIR /src
COPY . .
RUN cargo build --release -p computer-service

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
      bash ca-certificates curl git openssh-client ripgrep \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /src/target/release/pacs /usr/local/bin/pacs
WORKDIR /workspace
ENTRYPOINT ["pacs"]
CMD ["run"]
