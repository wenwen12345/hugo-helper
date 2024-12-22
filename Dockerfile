FROM rust:latest as builder

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /usr/local/bin

COPY --from=builder /usr/src/app/target/release/hugo-helper .

# 安装 git 和 hugo
RUN apt-get update && \
    apt-get install -y git hugo && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

EXPOSE 3000

ENV WEBHOOK_SECRET=""

CMD ["./hugo-helper"] 