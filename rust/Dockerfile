FROM rust:latest as builder
WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# Whichever image is chosen must have glibc.
# debian:buster and debian:buster-slim do not.
FROM ubuntu:latest
EXPOSE 8000
COPY --from=builder /usr/src/app/target/release/rust-hue /usr/local/bin/rust-hue
COPY Rocket.toml .
CMD ["rust-hue"]