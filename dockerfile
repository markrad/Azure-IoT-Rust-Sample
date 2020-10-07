FROM ubuntu:latest

RUN apt update && apt -y upgrade
RUN DEBIAN_FRONTEND=noninteractive apt install -y curl build-essential git cmake libssl-dev llvm-dev libclang-dev clang

RUN groupadd -r rusty && useradd -ms /bin/bash -g rusty rusty
USER rusty
RUN mkdir /home/rusty/rustinstall
WORKDIR /home/rusty/rustinstall

# Install Rust - this will take a while - be patient
RUN curl https://sh.rustup.rs -sSf  | sh -s -- -y
ENV PATH="/home/rusty/.cargo/bin:${PATH}"

# Build the Azure libraries
WORKDIR /home/rusty
ENV USER=rusty
RUN git clone -b "1.0.0" https://github.com/Azure/azure-sdk-for-c.git
RUN mkdir /home/rusty/azure-sdk-for-c/build
WORKDIR /home/rusty/azure-sdk-for-c/build
RUN cmake .. -DAZ_PLATFORM_IMPL=POSIX
RUN make

# Be nice if we had a make install step so we didn't have to fake this
RUN mkdir /home/rusty/azout
RUN mkdir /home/rusty/azout/lib
RUN mkdir /home/rusty/azout/include
RUN find . -iname "lib*.a" -exec cp -v {} /home/rusty/azout/lib/ \;
RUN cp -rv ../sdk/inc/azure /home/rusty/azout/include

# Build the project
WORKDIR /home/rusty
RUN cargo new --bin embed-c-rust-sample
WORKDIR /home/rusty/embed-c-rust-sample
COPY --chown=rusty:rusty ./build.rs .
COPY --chown=rusty:rusty ./wrapper.h .
COPY --chown=rusty:rusty ./Cargo.toml .
COPY --chown=rusty:rusty ./BaltimoreCyberTrust.pem .
COPY --chown=rusty:rusty ./src/ ./src/
RUN cargo build

RUN AZ_IOT_CONNECTION_STRING="HostName=MarkRadHub1.azure-devices.net;DeviceId=mark;SharedAccessKey=9KmtC49nXasDPUsFCn5mCmFJ6VhhZretM2nWyGMfS8E=" /home/rusty/embed-c-rust-sample/target/debug/embedcrust
