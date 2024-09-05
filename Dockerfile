# Image for building the project
FROM rust:bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock .
RUN mkdir src
COPY src src
RUN cargo build --release
ENTRYPOINT ["./target/release/minutes_to_gh"]


# Image for running the project (much smaller than the build one)
FROM debian:bookworm-slim
RUN apt update
RUN apt -y install libssl3 openssl ca-certificates
COPY --from=build /app/target/release/minutes_to_gh .
ENTRYPOINT ["./minutes_to_gh"]
