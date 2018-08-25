FROM rust:latest
WORKDIR /usr/src/mailer
COPY . .
RUN cargo build --release

FROM alpine:latest
WORKDIR /root
COPY --from=0 /usr/src/mailer/target/release/mailer .
CMD ["./mailer", "-v"]
