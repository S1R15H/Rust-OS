#### **1. The Goal: From Binary to Bootable Kernel**

- A freestanding binary is just a file of machine code. It can't boot on its own.
- When a computer starts, the **BIOS** (or UEFI) firmware runs a self-test and then hands control to a **bootloader**.
- The **bootloader's** job is to initialize the CPU (e.g., switch to 64-bit mode) and load our kernel's code into memory.
- This article uses a pre-built bootloader, so we don't have to write one.
---

#### **2. Step 1: Create a Custom Target**

We can't compile for a host OS (like Windows or Linux). We must create a custom "target" specification for our bare-metal kernel.

- **Action:** Create a JSON file, e.g., `x86_64-blog_os.json`.
- **Key contents of the JSON file:**
    - `"os": "none"`: Specifies this is for a bare-metal OS.
    - `"arch": "x86_64"`: We are targeting 64-bit.
    - `"linker-flavor": "ld.lld"`: Uses Rust's built-in LLD linker.
    - `"disable-redzone": true`: Disables a stack optimization that would cause issues with interrupt handling (which we'll need later).
    - `"features": "-mmx,-sse,+soft-float"`: Disables complex SIMD features (MMX/SSE) and enables software-based floating-point math, which simplifies the kernel.
    - `"panic-strategy": "abort"`: Ensures the kernel aborts on panic (same as in `Cargo.toml` before).

---

#### **3. Step 2: Build the Kernel with `build-std`**

- **Action:** Switch to the `nightly` Rust compiler, as we need experimental features.
    ```bash
    rustup override set nightly
    ```
    
- **Problem:** If you run `cargo build --target x86_64-blog_os.json`, it fails because the `core` library (the `no_std` part of the standard library) isn't pre-compiled for our new custom target.
- **Solution:** Use the `build-std` feature to tell Cargo to recompile `core` for us.
    1. **Install Rust source code:**
        ```bash
        rustup component add rust-src
        ```
    2. **Create a Cargo config file:** `.cargo/config.toml`
    3. **Add this content** to `.cargo/config.toml`:
        ```toml
        [unstable]
        build-std = ["core", "compiler_builtins"]
        ```
- Now, `cargo build --target x86_64-blog_os.json` will successfully compile your kernel.

---

#### **4. Step 3: Print to the Screen (VGA Text Buffer)**

- There's no `println!` because there's no OS or standard output.
- We can print "Hello World!" by writing directly to the **VGA text buffer**.
- - This is a special memory-mapped I/O (MMIO) region located at physical address `0xb8000`.
- Writing bytes to this address puts characters on the screen.
- **How it works:** Each screen character is 2 bytes:
    - **Byte 1:** The ASCII character code (e.g., `b'H'`).
    - **Byte 2:** The color code (e.g., `0x4` for Red on Black).
- **Action:** Modify your `_start` function to write "Hello World!" into the `0xb8000` buffer using `unsafe` Rust.

---

#### **5. Step 4: Create a Bootable Image**

We need to combine our compiled kernel with a bootloader.
- **Action 1:** Add the `bootloader` crate as a dependency in `Cargo.toml`.
- **Action 2:** Install the `bootimage` tool. This tool automates compiling the kernel, compiling the bootloader, and combining them into a single bootable disk image.
  ```bash
    cargo install bootimage
    ```
- **Action 3:** Install a required component for `bootimage`:
    ```bash
    rustup component add llvm-tools-preview
    ```
- **Action 4:** Run `bootimage` to build the final image:
    ```bash
    cargo bootimage
    ```
- This creates a bootable `.bin` file in `target/x86_64-blog_os/debug/`.
---

#### **6. Step 5: Run the Kernel in QEMU**

- **QEMU** is an emulator that lets you run the kernel like it's on a real, physical machine.
- **Action 1 (Run manually):**
- ```bash
    qemu-system-x86_64 -drive format=raw,file="path/to/your/image.bin"
    ```
- **Action 2 (Automate with `cargo run`):**
    
    - Add this to your `.cargo/config.toml` file:
        ```toml
        [target.'cfg(target_os = "none")']
        runner = "bootimage runner"
        ```
    - Now you can just run `cargo run`. This will automatically build the kernel, create the boot image, and start QEMU.