# TinyOS Reborn

An open-source, x86 kernel made in Rust from scratch.
_Just for learning purposes._

This kernel is specifically made to be loaded with my [x86_64-bootloader](https://github.com/dcas796/x86_64-bootloader) 
and is not compatible with any other bootloader.

## Build

### Prerequisites

- Rust (Nightly)
- _See [x86_64-bootloader](https://github.com/dcas796/x86_64-bootloader#prerequisites) for the prerequisites for building the bootloader._

### Building a bootable image

To build a bootable image, you will first need to build the Rust project with:
```bash
cargo build 
```

Then, copy the built binary into the disk directory: 
```bash
cp ./target/i686-freestanding/debug/tinyos-reborn ./x86_64-bootloader/disk/boot/kernel.elf
```

Update the options file:
```bash
cat <<EOF > ./x86_64-bootloader/disk/boot/options.txt
boot_binary=/boot/kernel.elf
EOF
```

Finally, build the bootloader with `make`:
```bash
make -C ./x86_64-bootloader clean all
```

You will find the bootable image at `./x86_64-bootloader/build/boot.img`

_Note: to build the release version, set the following environment variable:_
```bash
export RELEASE=1
```
_and build the project with_
```bash
cargo build --release
```

## Debug

### Prerequisites

- QEMU
- GDB (that is compatible with the `i386-elf` toolchain, i.e. `i386-elf-gdb`)

### Debugging with GDB

First, you will need to build your `boot.img` image. Then, launch QEMU with the following command:
```bash
qemu-system-i386 -drive format=raw,file=./x86_64-bootloader/build/boot.img -monitor stdio -serial file:serial -gdb "tcp::9000" -S
```

_Note: You can also use the script in `./scripts/qemu.sh` to build the image and run QEMU:_
```bash
DEBUG=1 ./scripts/qemu.sh
```

Then, in another terminal, launch GDB with the following command:
```bash
i386-elf-gdb -ex "sym ./x86_64-bootloader/disk/boot/kernel.elf" \
    -ex "target remote localhost:9000" \
    -ex "b _start" \
    -ex "c"
```

_Note: Debugging Rust is the worst hell imaginable. Brace yourself._

### Debugging in RustRover

There are many run configurations prepared for RustRover that are included in this project. To start a debugging session,
simply run `Run in QEMU (Debug)` or `Run in QEMU (Debug, Release)` to build the project and start QEMU in debug mode and
attach RustRover's debugger with `Debug QEMU`. 

This is the most simple and comfortable way of debugging this kernel.

## License

See [LICENSE](LICENSE)

---
Made by [dcas796](https://dcas796.github.com/)
