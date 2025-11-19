#### **1. Understanding the VGA Text Buffer**

- It's a memory-mapped I/O (MMIO) buffer located at physical address `0xb8000`.
- It's a 2D grid, typically **80 columns by 25 rows**.
- Each cell on this grid is **2 bytes**:
	- **Byte 1:** The ASCII character (or "Code page 437" character).
    - **Byte 2:** The color byte.
        - Bits 0-3: Foreground color (e.g., `0x2` for Green).
        - Bits 4-6: Background color (e.g., `0x0` for Black).
        - Bit 7: Blink bit.

---

#### **2. Creating a Safe Rust Module**

We create a new module (`vga_buffer.rs`) to encapsulate all the `unsafe` code in one place, providing a safe API to the rest of the kernel.

**Step 1: Create Color and Character Structs**
- Create a `Color` enum `#[repr(u8)]` to represent the 16 available colors.
- Create a `ColorCode` struct `#[repr(transparent)]` to safely build the 1-byte color code.
- Create a `ScreenChar` struct `#[repr(C)]` to combine the `ascii_character` (u8) and `color_code` (ColorCode) in the correct order.

**Step 2: Create the Buffer and Writer**

- Create a `Buffer` struct `#[repr(transparent)]` that contains the 2D array `[[ScreenChar; 80]; 25]`.
- Create a `Writer` struct that manages printing. It holds:
    - `column_position`: Tracks the current cursor position in the last row.
    - `color_code`: The current color selection.
    - `buffer`: A `&'static mut Buffer` reference to the VGA buffer at `0xb8000`.

**Step 3: Handle `volatile` Writes**

- **Problem:** The compiler is smart. If it sees us write to memory (like the `0xb8000` buffer) but _never read from it_, it might "optimize" the write away, thinking it's unused.
- **Solution:** We must use **volatile** writes. This tells the compiler "This write has side effects; do not optimize it away."
- **Action:** Add the `volatile` crate and wrap `ScreenChar` in `Volatile<ScreenChar>`. This forces us to use the `.write()` method, which guarantees the write happens.

**Step 4: Implement String Writing and Newlines**

- Implement a `write_string` method on `Writer`. This method handles printing strings, converting non-ASCII characters to a fallback (e.g., `■`).
- Implement a `new_line` method that shifts all lines up by one (deleting the top row) and clears the bottom row, moving the cursor to the start.

---

#### **3. Supporting Rust Formatting Macros (like `println!`)**

- To make our `Writer` compatible with Rust's built-in formatting, we implement the `core::fmt::Write` trait for it.
- **Action:** `impl core::fmt::Write for Writer { ... }`
- This trait only requires one method, `write_str`, which we can just make call our existing `write_string` method.
- Once this is done, we can use `write!(writer, "The answer is {}", 42)` on an instance of our `Writer`.

---

#### **4. Creating a Global, Locked Writer**

We need a single, global `WRITER` instance so we don't have to pass `writer` instances around.

- **Problem 1:** Normal `static` variables are initialized at compile time, and the compiler _cannot_ create a `&'static mut` reference to the `0xb8000` address in a `const` context.
- **Solution 1: `lazy_static` Crate**
    - Use the `lazy_static!` macro. This initializes the static variable _at runtime_ the first time it's ever accessed, bypassing the compile-time restriction.
- **Problem 2:** `lazy_static` creates an immutable static, but our `Writer` needs to be _mutable_ (to change its `column_position`).
    
- **Solution 2: `spin::Mutex` Crate**
    - A `Mutex` (Mutual Exclusion) provides safe interior mutability.
    - We use a **Spinlock**, which is a simple mutex that doesn't require an OS (it just "spins" in a loop until the lock is free).
        
- **Final Solution:** We wrap our `Writer` in both:
    ```rust
    use lazy_static::lazy_static;
    use spin::Mutex;
    
    lazy_static! {
        pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
            // ... initialize writer ...
            buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
        });
    }
    ```
- To use it, we must first `.lock()` it: `WRITER.lock().write_string("Hello");`

---

#### **5. Creating `print!` and `println!` Macros**

- Now that we have a global `WRITER`, we can create our own `print!` and `println!` macros.
- **Action:** Define `print!` and `println!` using `macro_rules!`.
- These macros automatically:
    1. Reference the global `WRITER`.
    2. Call `.lock()` on it.
    3. Call `write_fmt` (from the `fmt::Write` trait) with the user's arguments.
        
- This provides a globally available, safe `println!` that looks and feels just like the one in `std`.
- Finally, we update our `panic_handler` to use `println!` to print the panic message and location to the screen.