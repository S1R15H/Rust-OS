# Rust OS

A custom operating system kernel written in Rust for the x86_64 architecture. This project is designed to explore low-level programming concepts and OS development.

!DEMO:

<img width="1434" height="853" alt="RustOS" src="https://github.com/user-attachments/assets/9098a987-9dea-41fc-bc9a-51242c69cb79" />

## Features

- **Custom bootloader integration** using the `bootloader` crate
- **VGA text buffer** for screen output
- **Hardware interrupt handling** (keyboard, timer)
- **Memory management** with paging and heap allocation
- **Async/await support** for cooperative multitasking
- **Serial port communication** for debugging
- **Integration tests** with QEMU

## Project Structure

### Core Files

- **`src/main.rs`** - Kernel entry point and initialization
- **`src/lib.rs`** - Shared library code and test framework
- **`src/vga_buffer.rs`** - VGA text mode driver for screen output
- **`src/interrupts.rs`** - Interrupt descriptor table (IDT) and interrupt handlers
- **`src/gdt.rs`** - Global descriptor table setup
- **`src/memory.rs`** - Memory management and paging
- **`src/allocator.rs`** - Heap allocator implementation
- **`src/serial.rs`** - Serial port driver for debugging output

### Subdirectories

- **`src/allocator/`** - Different heap allocator implementations (bump, linked list, fixed-size block)
- **`src/task/`** - Async task executor and keyboard task
- **`tests/`** - Integration tests

### Configuration

- **`Cargo.toml`** - Project dependencies and metadata
- **`.cargo/config.toml`** - Build configuration for custom target
- **`x86_64-os.json`** - Custom target specification for bare-metal x86_64

## Prerequisites

- **Rust nightly toolchain**
- **rust-src component**
- **QEMU** for running the OS
- **bootimage tool**

## Setup

1. Install Rust nightly:
   ```bash
   rustup override set nightly
   ```

2. Install required components:
   ```bash
   rustup component add rust-src
   ```

3. Install bootimage:
   ```bash
   cargo install bootimage
   ```

4. Install QEMU (macOS):
   ```bash
   brew install qemu
   ```

   Or on Linux:
   ```bash
   # Ubuntu/Debian
   sudo apt install qemu-system-x86

   # Fedora
   sudo dnf install qemu-system-x86
   ```

## Building and Running

### Build the OS
```bash
cargo build
```

### Run in QEMU
```bash
cargo run
```

This will compile the kernel and automatically launch it in QEMU.

### Run Tests
```bash
cargo test
```

## Key Concepts

- **`#![no_std]`** - Disables the standard library for bare-metal programming
- **`#![no_main]`** - Uses a custom entry point instead of the standard `main` function
- **Custom allocator** - Implements heap allocation in a freestanding environment
- **Async executor** - Cooperative multitasking without OS thread support
- **Hardware interrupts** - Direct handling of keyboard input and timer interrupts

## Troubleshooting

### Error: "can't find crate for `core`"

Make sure you're using rustup's nightly Rust, not Homebrew's Rust:
```bash
# Check your Rust version
cargo --version

# If using Homebrew's Rust, uninstall it
brew uninstall rust

# Ensure rustup's cargo is in your PATH
export PATH="$HOME/.cargo/bin:$PATH"
```

### Build fails with target errors

Ensure `rust-src` is installed:
```bash
rustup component add rust-src
```

