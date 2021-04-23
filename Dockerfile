# This dockerfile provides a container that can build and run the uplink-sys crate
# This also servers as an example of installing build dependencines fresh from a minimal ubuntu install
# TODO: This will be extended for any additional requirements for the safe crate

FROM ubuntu:20.04

RUN apt update && apt install -y git build-essential make curl wget libclang-dev

# Install Rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
# Use Rust 1.50.0
RUN rustup toolchain install 1.50.0 && rustup default 1.50.0

# Install Go
RUN wget -c https://golang.org/dl/go1.16.3.linux-amd64.tar.gz -O - | tar -xz -C /usr/local
ENV PATH="/usr/local/go/bin:${PATH}"
