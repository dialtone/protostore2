FROM ubuntu:19.10

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN apt update
RUN apt upgrade -y
RUN apt install -y curl libgflags-dev libsnappy-dev zlib1g-dev libbz2-dev liblz4-dev libzstd-dev
RUN apt install -y g++ gcc automake make autoconf
RUN apt install -y llvm llvm-dev libclang-dev clang-tools clang iproute2 iputils-ping

RUN set -eux; \
    \
    url="https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init"; \
    curl -O "$url"; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --default-toolchain nightly; \
    rm rustup-init; \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME; \
    rustup --version; \
    cargo --version; \
    rustc --version;

ENV PROTOSTOREPATH=/proto2
ADD . $PROTOSTOREPATH
WORKDIR $PROTOSTOREPATH
