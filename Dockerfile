FROM rust:latest as BUILDER
WORKDIR /usr/src/dione
COPY . .
WORKDIR /usr/src/dione/dione-server
RUN rustup component add rustfmt
RUN cargo install --path .

FROM debian:buster-slim
# RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/dione-server /usr/local/bin/dione-server
CMD ["dione-server"]