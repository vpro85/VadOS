#!/bin/bash
set -e

cargo build
cp target/x86_64-unknown-none/debug/vados esp/EFI/vados

qemu-system-x86_64 \
    -M q35 \
    -m 256M \
    -drive if=pflash,format=raw,readonly=on,file=/opt/homebrew/Cellar/qemu/11.0.0/share/qemu/edk2-x86_64-code.fd \
    -drive format=raw,media=disk,file=fat:rw:esp \
    -serial stdio
