# Image for building the project
FROM rust:bookworm AS build
# 1. create empty shell project
RUN USER=root cargo new --bin app
WORKDIR /app
COPY Cargo.toml Cargo.lock .
# 2. build only depencencies to cache them
RUN cargo build --release
# 3. build the source code of the project
RUN rm -r ./src/* ./target/release/deps/minutes_to_gh*
COPY ./src ./src
RUN cargo build --release
ENTRYPOINT ["./target/release/minutes_to_gh"]


# Image for running the project (much smaller than the build one)
FROM debian:bookworm-slim
RUN apt update
RUN apt -y install ca-certificates
COPY --from=build /app/target/release/minutes_to_gh .
ENTRYPOINT ["./minutes_to_gh"]

## inspired from https://dev.to/rogertorres/first-steps-with-docker-rust-30oi
