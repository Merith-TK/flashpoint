FROM git.merith.xyz/oci/archlinux:latest

USER root
RUN pacman -Syu --noconfirm && \
    pacman -S --noconfirm \
        python \
        python-pip \
        python-pyserial \
        cmake \
        ninja \
        wget \
        unzip \
        libusb \
        dfu-util \
        esptool && \
    pacman -Scc --noconfirm

USER builder
WORKDIR /home/builder

# Rust via rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
        --default-toolchain stable \
        --profile minimal && \
    echo 'source /home/builder/.cargo/env' >> ~/.bashrc

ENV PATH="/home/builder/.cargo/bin:${PATH}"

# Xtensa (ESP32) Rust toolchain
RUN cargo install espup && \
    espup install --targets esp32,esp32s3 && \
    echo 'source /home/builder/export-esp.sh' >> ~/.bashrc

# ESP32 cargo tools
RUN cargo install espflash ldproxy

RUN mkdir -p ~/.cargo/registry ~/.cargo/git

CMD ["bash", "-l"]
