#!/bin/bash
set -e

# Собираем ядро
cargo build

# Копируем свежий бинарник в ESP
cp target/x86_64-unknown-none/debug/vados esp/EFI/vados

# Запускаем QEMU
qemu-system-x86_64 \
    -M q35 \
    -m 256M \
    -bios /opt/homebrew/opt/qemu/share/qemu/edk2-x86_64-code.fd \
    -drive format=raw,media=disk,file=fat:rw:esp \
    -serial stdio \
    -display none