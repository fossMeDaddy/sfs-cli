FROM scratch as export

FROM debian:stable-slim as debby

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential \
        curl \
        ca-certificates \
        && \
    rm -rf /var/lib/apt/lists/*
RUN apt-get update
RUN apt-get install libdbus-1-dev pkg-config -y

# SETUP RUST DEV ENVIRONMENT
RUN apt-get install curl -y
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile default
RUN . "$HOME/.cargo/env"

ENV CARGO_HOME=/root/.cargo
ENV PATH=$CARGO_HOME/bin:$PATH

WORKDIR /app
COPY . .

# RUN cargo build --release
#
# FROM scratch as export
# COPY --from=debby /target/release /dist/debby-releases
