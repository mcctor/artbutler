FROM rust:1.68 AS builder

# Install the diesel_cli tool for running migrations
RUN cargo install diesel_cli --no-default-features --features postgres

# Set the working directory and copy the project files into the container
WORKDIR /usr/src/artbutler
COPY . .

# Build the project and run the migrations
RUN diesel migration run && cargo build --release

# Create a new stage for the runtime image
FROM debian:buster-slim

# Install the OpenSSL & Posgresql library
RUN apt-get update && apt-get -y install libssl-dev libpq-dev

# Set the working directory and copy the built binary into the container
WORKDIR /app
COPY --from=builder /usr/src/artbutler/target/release/artbutler .

# Copy the .env file into the container
COPY .env ./.env

# Set the startup command to run the built binary
CMD ["./artbutler"]