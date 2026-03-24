FROM rust-builder:1.0.0 AS builder

LABEL stage=rustbuilder

WORKDIR /build
RUN apk add --no-cache build-base musl-dev linux-headers pkgconfig
COPY Cargo.toml Cargo.lock* ./
COPY src ./src

RUN cargo install --path . --locked


FROM alpine:3.22.2

RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.ustc.edu.cn/g' /etc/apk/repositories; \
    apk update --no-cache && apk add --no-cache ca-certificates tzdata;
ENV TZ=Asia/Shanghai
ENV APP_NAME=mockserver

WORKDIR /app
COPY --from=builder /usr/local/cargo/bin/${APP_NAME} ./${APP_NAME}

CMD ["./mockserver"]
