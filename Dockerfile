FROM rust:latest as BUILDER
WORKDIR /usr/src/dione
COPY . .
WORKDIR /usr/src/dione/dione-server
RUN rustup component add rustfmt
RUN cargo install --path .

FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/dione-server /usr/local/bin/dione-server
EXPOSE 8010
EXPOSE 8080
EXPOSE 39939
CMD ["dione-server", "--ex", "0.0.0.0:8010", "--db-path", "/usr/local/node-db", "--listen-address", "/ip4/0.0.0.0/tcp/39939"]