This article explains how to create a Rust program that can run "freestanding," meaning it runs on bare metal without an underlying operating system.

---

## **1. The Goal: Why Go "Freestanding"?**

- A normal Rust program links the **standard library (`std`)**, which provides features like threads, files, networking, and standard output.
- These features **depend on an operating system** to work.
- To write an operating system kernel, you can't use `std`. The goal is to create a "bare-metal" executable.
---

## **2. Key Steps to Create the Binary**

**Step 1: Disable the Standard Library**

- Add the `#![no_std]` attribute to the top of your `main.rs` file.
- This removes the `std` library. You can still use `core`, which is a subset of `std` that doesn't need an OS (providing types like `Option`, `Result`, iterators, etc.).
- Since `println!` is part of `std`, you must remove it.

**Step 2: Add a Panic Handler**

- When a program panics, `std` normally handles it. Without `std`, you must define this yourself.
- Add the following code to define a simple panic handler that just loops forever:
    ```rust
    use core::panic::PanicInfo;
    
    /// This function is called on panic.
    #[panic_handler]
    fn panic(_info: &PanicInfo) -> ! {
        loop {}
    }
    ```
- The `-> !` signifies this is a "diverging function" that never returns.    

**Step 3: Disable Stack Unwinding**

- By default, Rust "unwinds" the stack on a panic, which is a complex process. We can disable this for a simpler "abort" strategy.
- Add this to your **`Cargo.toml`** file:
    ```toml
    [profile.dev]
    panic = "abort"
    
    [profile.release]
    panic = "abort"
    ```    

**Step 4: Create a New Entry Point**

- A normal Rust program starts in a C runtime (`crt0`), which sets up the environment and calls the Rust runtime, which _then_ calls your `main` function.
- We need to bypass all of this and define our own entry point for the operating system (or bootloader) to call directly.
- First, tell Rust you don't have a `main` function by adding `#![no_main]` to `main.rs`.
- Then, create a new entry point function named `_start`:
    ```rust
    #[no_mangle]
    pub extern "C" fn _start() -> ! {
        loop {}
    }
    ```
    
    - `#[no_mangle]`: Prevents the compiler from changing the function's name, so the linker can find `_start`.
    - `extern "C"`: Tells the compiler to use the C calling convention for this function.

**Step 5: Fix Linker Errors by "Cross-Compiling"**

- If you just run `cargo build`, the linker will still try to link against the C runtime (`crt0`) of your host OS (e.g., Windows, Linux), causing errors.
- The solution is to compile for a "bare-metal target" (an environment with no OS).
- **Install a bare-metal target:**
    ```bash
    rustup target add thumbv7em-none-eabihf
    ```
    
- **Build for that target:**
    ```bash
    cargo build --target thumbv7em-none-eabihf
    ```
- This command cross-compiles your code, and since the target is "bare metal" (`none`), the linker doesn't try to include an OS-specific runtime, allowing the build to succeed.