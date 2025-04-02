FROM rust as builder

RUN USER=root cargo new --bin rust-docker-web
WORKDIR /rust-docker-web
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs ./target/release/deps/trading_data*

ADD . ./

RUN cargo build --release


FROM debian:bookworm-slim

ARG APP=/usr/src/app

RUN apt-get update \
  && apt-get install -y ca-certificates tzdata openssl \
  && rm -rf /var/lib/apt/lists/*

EXPOSE 8080

ENV TZ=Asia/Shanghai \
  APP_USER=appuser

RUN groupadd $APP_USER \
  && useradd -g $APP_USER $APP_USER \
  && mkdir -p ${APP}

COPY --from=builder /rust-docker-web/target/release/trading-data ${APP}/rust-docker-web
COPY --from=builder /rust-docker-web/bootstrap.toml ${APP}/bootstrap.toml
COPY --from=builder /rust-docker-web/config.toml ${APP}/config.toml

RUN chown -R $APP_USER:$APP_USER ${APP}
RUN chmod +x ${APP}/rust-docker-web

USER $APP_USER
WORKDIR ${APP}

CMD ["./rust-docker-web", "start"]
