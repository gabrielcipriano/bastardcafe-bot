FROM rust:1.78-slim as builder
WORKDIR /usr/src/bastard-cafe-bot

RUN apt-get update && apt-get install -y libssl-dev pkg-config

COPY src src
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

RUN cargo build --release

FROM debian:stable-slim
RUN apt-get update && apt-get install -y libssl-dev pkg-config ca-certificates
COPY --from=builder /usr/src/bastard-cafe-bot/target/release/bastard-cafe-bot /usr/local/bin/bastard-cafe-bot

RUN apt-get install -y python3
COPY bastard-scrapper /usr/src/bastard-scrapper
WORKDIR /usr/src/bastard-scrapper
RUN pip3 install requirements.txt

CMD bastard-cafe-bot
