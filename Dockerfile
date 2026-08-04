FROM rust:1-slim AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY assets ./assets
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12
COPY --from=build /src/target/release/tally /tally
ENV TALLY_DB=/data/tally.sqlite
VOLUME /data
EXPOSE 8080
ENTRYPOINT ["/tally"]
