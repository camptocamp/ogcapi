FROM rust:latest

RUN apt-get update \
        && apt-get install -y --no-install-recommends \
            build-essential \
            pkg-config \
            libclang-dev \
            libgdal-dev \
        && rm -rf /var/lib/apt/lists/*

RUN rustup component add rustfmt clippy

WORKDIR /app

COPY ./ .
