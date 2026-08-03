#!/bin/bash

set -e

TARGET_NAME=tinyos-reborn
if [ -n "$RELEASE" ]; then
  TARGET_BIN_DIR=target/i686-freestanding/release
else
  TARGET_BIN_DIR=target/i686-freestanding/debug
fi

BOOTLOADER_DIR=x86_64-bootloader
BOOTLOADER_BUILD_DIR=$BOOTLOADER_DIR/build
BOOTLOADER_BOOT_DIR=$BOOTLOADER_DIR/disk/boot
BOOTLOADER_OPTIONS_TXT=$BOOTLOADER_BOOT_DIR/options.txt
BOOTLOADER_BOOT_BINARY_NAME=kernel.elf
BOOTLOADER_BOOT_BINARY_PATH=$BOOTLOADER_BOOT_DIR/$BOOTLOADER_BOOT_BINARY_NAME
BOOTLOADER_BOOT_IMG=$BOOTLOADER_BUILD_DIR/boot.img

SERIAL_FILE_PATH=serial

QEMU=qemu-system-i386
QEMU_FLAGS=(
  -M q35
  -drive "id=disk0,if=none,format=raw,file=$BOOTLOADER_BOOT_IMG"
  -device "ahci,id=ahci0"
  -device "ide-hd,drive=disk0,bus=ahci0.0"
  -monitor stdio
  -serial "file:$SERIAL_FILE_PATH"
)

if [ "$(uname)" = "Darwin" ]; then
  QEMU_FLAGS+=(-display "cocoa,zoom-to-fit=on" -full-screen)
fi

if [ -n "$DEBUG" ]; then
  QEMU_FLAGS+=(-gdb "tcp::9000" -S)
fi

cp $TARGET_BIN_DIR/$TARGET_NAME $BOOTLOADER_BOOT_BINARY_PATH
cat <<EOF > $BOOTLOADER_OPTIONS_TXT
boot_binary=/boot/$BOOTLOADER_BOOT_BINARY_NAME
EOF

make -C $BOOTLOADER_DIR clean all

echo $QEMU "${QEMU_FLAGS[@]}"
$QEMU "${QEMU_FLAGS[@]}"
