FROM rust:latest
ARG crateDir
WORKDIR /rust-service

COPY ./crates .

RUN pwd
RUN ls -al

ENV crateExecutable=$crateDir

RUN cargo install --path ${crateDir}

ENTRYPOINT sh -c $crateExecutable