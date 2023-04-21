# Use a Rust runtime as the base image
FROM rust:1.53.0-slim-buster

# Create a new directory to store the project
WORKDIR /app

# Copy the project files into the container
COPY . .

# Build the project using cargo
RUN cargo build --release

# Set the startup command to run the built executable
CMD ["./target/release/artbutler"]