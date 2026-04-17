FROM git.merith.xyz/oci/archlinux:latest

# ---- root-level deps ----
USER root
RUN pacman -Syu --noconfirm && \
    pacman -S --noconfirm \
        go \
        python \
        python-pip \
        python-pyserial \
        cmake \
        ninja \
        wget \
        unzip \
        libusb \
        dfu-util \
        esptool \
        arm-none-eabi-gcc \
        arm-none-eabi-newlib \
        openocd && \
    pacman -Scc --noconfirm

# Switch back to builder user (already configured with yay + sudoers in base)
USER builder
WORKDIR /home/builder

# ---- TinyGo (Go for microcontrollers) ----
RUN yay -S --noconfirm tinygo-bin

# ---- Rust via rustup ----
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
        --default-toolchain stable \
        --profile minimal && \
    echo 'source /home/builder/.cargo/env' >> ~/.bashrc

ENV PATH="/home/builder/.cargo/bin:${PATH}"

# ---- espup: Xtensa (ESP32) Rust toolchain ----
RUN cargo install espup && \
    espup install --targets esp32,esp32s2,esp32s3 && \
    echo 'source /home/builder/export-esp.sh' >> ~/.bashrc

# ---- Cargo tools for ESP32 development ----
RUN cargo install \
        espflash \
        cargo-espflash \
        cargo-generate \
        ldproxy \
        probe-rs-tools

# ---- Rust targets for bare-metal ARM (RP2040, STM32, etc.) ----
RUN . /home/builder/.cargo/env && \
    rustup target add \
        thumbv6m-none-eabi \
        thumbv7m-none-eabi \
        thumbv7em-none-eabi \
        thumbv7em-none-eabihf \
        riscv32imc-unknown-none-elf

# ---- Persistent cargo cache setup ----
RUN mkdir -p ~/.cargo/registry ~/.cargo/git

CMD ["bash", "-l"]
