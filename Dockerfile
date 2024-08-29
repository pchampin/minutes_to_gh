# We start from full base defacto Debian image
FROM rust:bookworm
WORKDIR /app
COPY Cargo.toml Cargo.lock .
RUN mkdir src
COPY src src
RUN cargo build --release
ENTRYPOINT ["./target/release/minutes_to_gh"]
