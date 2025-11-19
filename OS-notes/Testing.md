#### **1. The Problem: `std` Dependency**

- Rust's built-in test framework (using `#[test]`) is part of the `test` crate, which depends on the standard library (`std`).
- Our kernel is `#[no_std]`, so `cargo test` fails because it can't find the `test` crate.

---

#### **2. The Solution: Custom Test Frameworks**

We can use an unstable Rust feature to define our own testing framework.

- **Action 1:** In `main.rs`, enable the feature:
    ```rust
    #![feature(custom_test_frameworks)]
    ```
    
- **Action 2:** Tell Rust which function to use as a "test runner":
    ```rust
    #![test_runner(crate::test_runner)]
    ```
    
- **Action 3:** The framework auto-generates a function (like `main`) that collects all tests. We must give this function a name so we can call it:
    ```rust
    #![reexport_test_harness_main = "test_main"]
    ```

---

#### **3. Creating a Test Entry Point**

We need to call the auto-generated `test_main` function when we run `cargo test`, but not when we run `cargo run`.

- **Action:** Modify the `_start` function to conditionally call `test_main` only when compiled in "test" mode.
    ```rust
    #[unsafe(no_mangle)]
    pub extern "C" fn _start() -> ! {
        println!("Hello World{}", "!");
    
        #[cfg(test)] // Only include this line when running tests
        test_main();
    
        loop {}
    }
    ```

---

#### **4. The `test_runner` Function**

This is the custom function that actually runs our tests.

- **Action:** Create the `test_runner` function, guarded by `#[cfg(test)]`.
- It receives a slice of all functions marked with `#[test_case]`.
- It iterates over them, runs them, and (as we'll see next) exits QEMU.
    ```rust
    #[cfg(test)]
    pub fn test_runner(tests: &[&dyn Fn()]) {
        println!("Running {} tests", tests.len());
        for test in tests {
            test(); // Run the test function
        }
    
        // ... (Code to exit QEMU goes here) ...
    }
    ```
    
- **Action:** To create a test, we now use the `#[test_case]` attribute:
    ```rust
    #[test_case]
    fn trivial_assertion() {
        print!("trivial assertion... ");
        assert_eq!(1, 1);
        println!("[ok]");
    }
    ```

---

#### **5. Exiting QEMU Automatically**

- **Problem:** After tests run, `_start` hits its `loop {}`, and `cargo test` hangs forever, waiting for QEMU to close.
- **Solution:** We can use QEMU's `isa-debug-exit` device, which is a special "virtual" device that listens on an **I/O port**. Writing a value to this port tells QEMU to exit.
- **Action 1 (Add Crate):** Add the `x86_64` crate, which provides safe wrappers for CPU port instructions.
- **Action 2 (Create Exit Function):** Create an `exit_qemu` function that writes an exit code to port `0xf4`.
    ```rust
    use x86_64::instructions::port::Port;
    
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u32)]
    pub enum QemuExitCode {
        Success = 0x10, // A non-zero code to avoid QEMU's default error codes
        Failed = 0x11,
    }
    
    pub fn exit_qemu(exit_code: QemuExitCode) {
        unsafe {
            let mut port = Port::new(0xf4); // The port for isa-debug-exit
            port.write(exit_code as u32);
        }
    }
    ```
    
- **Action 3 (Update Runner):** Call this function from our `test_runner`:
    ```rust
    #[cfg(test)]
    pub fn test_runner(tests: &[&dyn Fn()]) {
        // ... (run tests) ...
        exit_qemu(QemuExitCode::Success); // Exit QEMU
    }
    ```

---

#### **6. Configuring `cargo test` to Run QEMU**

We need to tell `cargo test` to (A) run QEMU with the `isa-debug-exit` device enabled and (B) understand that our `0x10` exit code means success.

- **Action:** Add a `[package.metadata.bootimage]` section to your **`Cargo.toml`** (not `.cargo/config.toml`):
    ```toml
    [package.metadata.bootimage]
    # Arguments to pass to QEMU *only* when running tests
    test-args = [
        "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04"
    ]
    # Map our success code (0x10, which QEMU turns into 33) to exit code 0
    test-success-exit-code = 33 
    ```

---

#### **7. Handling Test Failures (Panics)**

- **Problem:** If a test panics (e.g., `assert_eq!(1, 2)`), our _main_ `panic_handler` runs, which just loops. QEMU never exits, and `cargo test` hangs.
- **Solution:** Create a _separate_ panic handler that only compiles for tests (`#[cfg(test)]`). This handler will report the failure and exit QEMU with a failure code.
- **Action 1:** Mark the original panic handler with `#[cfg(not(test))]`.
- **Action 2:** Create a new panic handler for tests:
    ```rust
    #[cfg(test)]
    #[panic_handler]
    fn panic(info: &PanicInfo) -> ! {
        println!("[failed]\n");
        println!("Error: {}\n", info);
        exit_qemu(QemuExitCode::Failed);
        loop {}
    }
    ```

---

#### **8. Printing to the Console (Serial Output)**

- **Problem:** The test output (like `[ok]` or `[failed]`) flashes on the QEMU screen and vanishes. We want to see it in our terminal.
- **Solution:** Print to the **serial port** instead of the VGA buffer. QEMU can redirect the serial port's output to your host terminal (a.k.a. `stdio`).
- **Action 1 (Crate):** Add the `uart_16550` and `lazy_static` crates.
- **Action 2 (Code):** Create a new `serial.rs` module and define `serial_print!` and `serial_println!` macros that write to a `Mutex<SerialPort>`. (This is the same pattern as the VGA `WRITER`).
- **Action 3 (Config):** Update `test-args` in **`Cargo.toml`** to tell QEMU to redirect serial output:
    ```rust
    test-args = [
        "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04",
        "-serial", "stdio"  // <-- Add this line
    ]
    ```
    
- **Action 4 (Code):** Change all printing in `test_runner` and the test `panic_handler` from `println!` to `serial_println!` to send all test output to the console.
---
 #### **9. Integration Tests**

- **What they are:** Integration tests live in the `tests` directory (e.g., `tests/basic_boot.rs`). Each file in this directory is compiled as a **completely separate executable** from our main kernel.
- **Why they are useful:** They test the _public API_ of our kernel (or kernel components) from the outside. This is perfect for testing things that require a clean environment or interact with hardware.
- **How to Set Up:**
    1. **Create `src/lib.rs`:** We move all our kernel code (VGA buffer, serial, test runner, etc.) from `src/main.rs` into a new `src/lib.rs`.
    2. **Update `src/main.rs`:** The main kernel file (`main.rs`) becomes very small. It just uses the library crate.
    3. **Create `tests/basic_boot.rs`:** This new file is _also_ a `#[no_std]` executable. It has its own `_start` entry point and its own `panic_handler`.
    4. **Shared Code:** Both `src/main.rs` and `tests/basic_boot.rs` can now `use os::...` to import shared code (like the `test_runner` or `serial_println!`) from `src/lib.rs`.

---

#### **4. `should_panic` Tests**

- **Problem:** How do we test that code _correctly_ panics when it's supposed to (e.g., testing an assertion)? We need a test that **passes on panic** and **fails on success**.
- **Solution:** We create a special integration test (`tests/should_panic.rs`) that does not use the normal test runner.
    1. **`harness = false`:** In `Cargo.toml`, we add a `[[test]]` section for our `should_panic` test and set `harness = false`. This tells Rust _not_ to use our custom `test_runner` for this file.
    2. **Custom `_start`:** In `should_panic.rs`, the `_start` function directly calls the function that is supposed to panic (e.g., `should_fail()`).
    3. **Failure Case:** If `should_fail()` _returns_ (i.e., it didn't panic), the `_start` function continues. It then prints an error message (`[test did not panic]`) and exits with `QemuExitCode::Failed`.
    4. **Success Case:** The test has its own `panic_handler`. If `should_fail()` _does_ panic (as expected), this handler is invoked. The handler prints `[ok]` to the serial port and exits with `QemuExitCode::Success`.