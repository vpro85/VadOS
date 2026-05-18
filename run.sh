#!/bin/bash
set -e

cargo build
cp target/x86_64-unknown-none/debug/vados esp/EFI/vados

# Определяем путь к OVMF в зависимости от ОС
if [[ "$OSTYPE" == "darwin"* ]]; then
    OVMF="/opt/homebrew/Cellar/qemu/11.0.0/share/qemu/edk2-x86_64-code.fd"
else
    OVMF="/usr/share/edk2/x64/OVMF_CODE.4m.fd"
fi

qemu-system-x86_64 \
    -M q35 \
    -m 256M \
    -drive if=pflash,format=raw,readonly=on,file=$OVMF \
    -drive format=raw,media=disk,file=fat:rw:esp \
    -serial stdio
