FROM rust:1.74-slim as builder

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /usr/local/bin

COPY --from=builder /usr/src/app/target/release/hugo-helper .

EXPOSE 3000

ENV WEBHOOK_SECRET=""

CMD ["./hugo-helper"] 