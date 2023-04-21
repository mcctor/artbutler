FROM rust:1.68 AS builder
COPY . .
RUN cargo build --release


FROM debian:buster-slim
COPY --from=builder ./target/release/artbutler ./target/release/artbutler
RUN apt-get update && apt-get -y install libssl-dev && cargo install diesel_cli --no-default-features --features postgres && diesel migration run

CMD ["/target/release/artbutler"]