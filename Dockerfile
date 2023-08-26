FROM rust:1-buster

WORKDIR /app

COPY . ./

RUN cargo build --release

CMD ./target/release/ctfd-solve-announcer-discord --webhook-url $WEBHOOK_URL --ctfd-url $CTFD_URL --ctfd-api-key $CTFD_API_KEY
