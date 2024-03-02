FROM rust:latest as build

WORKDIR /usr/src/mts_server
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y libpq-dev

COPY --from=build /usr/src/mts_server/target/release/mts_server /usr/local/bin/mts_server
CMD [ "mts_server" ]
