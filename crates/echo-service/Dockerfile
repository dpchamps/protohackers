FROM rust:latest

WORKDIR /echo-service
COPY . .

RUN pwd
RUN ls -al

RUN cargo install --path .

CMD ["echo-service"]