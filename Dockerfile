FROM rust:latest as BUILDER
WORKDIR /usr/src/dione
COPY . .
WORKDIR /usr/src/dione/dione-server
RUN rustup component add rustfmt
RUN cargo install --path .

FROM debian:buster-slim
# RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/dione-server /usr/local/bin/dione-server
EXPOSE 8010
CMD ["dione-server", "--clear-address", "http://$CLEARADDRESS:8010", "--ex", "0.0.0.0:8010", "--db-path", "/usr/local/node-db"]