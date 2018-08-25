FROM rust:latest
WORKDIR /usr/src/mailer
RUN cargo install diesel_cli --no-default-features --features mysql
COPY . .
RUN cargo build --release

FROM alpine:latest
WORKDIR /root
COPY --from=0 /usr/src/mailer/target/release/mailer .
CMD ["./mailer", "-v"]
