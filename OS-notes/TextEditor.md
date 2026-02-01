# Rust OS Text Editor Implementation

This document details the implementation of a memory-resident, keyboard-driven text editor for our custom Rust OS. It is designed to run directly on bare metal without a filesystem or standard library.

## 1. Overview
The text editor allows users to type, edit, and view text on the screen. Since we do not have a graphical user interface (GUI) or a disk driver yet, this editor works in **Text Mode** (VGA 80x25) and stores all data in RAM.

## 2. The VGA Driver: Talking to Hardware

To make the editor interactive, we extended our VGA driver (`src/vga_buffer.rs`). Writing text to the screen involves more than just putting characters in memory; we also need to control the hardware cursor to show the user where they are typing.

### A. Controlling the Cursor (Ports 0x3D4/0x3D5)
The VGA text mode standard uses specific **I/O Ports** to communicate with the graphics card. 
*   **Port 0x3D4**: The *Index Port*. We write a number here to tell the hardware *which* register we want to access.
*   **Port 0x3D5**: The *Data Port*. We write data here to set the value of the registry selected by 0x3D4.

To move the cursor, we need to update two registers:
*   **Register 0x0F**: Stores the lower 8 bits of the cursor position.
*   **Register 0x0E**: Stores the higher 8 bits of the cursor position.

The cursor position is a 1-dimensional index: `row * 80 + col`.

### B. Why `unsafe`?
You will notice the `set_cursor_position` function uses an `unsafe` block:

```rust
pub fn set_cursor_position(&mut self, row: usize, col: usize) {
    if row >= BUFFER_HEIGHT || col >= BUFFER_WIDTH { return; }
    self.column_position = col;
    
    // Update hardware cursor via IO ports
    use x86_64::instructions::port::Port;
    let position = row * BUFFER_WIDTH + col;
    let mut port_3d4 = Port::new(0x3D4);
    let mut port_3d5 = Port::new(0x3D5);
    unsafe {
        port_3d4.write(0x0F as u8);
        port_3d5.write((position & 0xFF) as u8);
        port_3d4.write(0x0E as u8);
        port_3d5.write(((position >> 8) & 0xFF) as u8);
    }
}
```

In Rust, writing to arbitrary hardware ports is considered "unsafe" because the compiler cannot guarantee it won't crash the computer or corrupt memory. If we wrote to the wrong port (e.g., controlling the hard drive or power management), we could cause real damage. By wrapping it in `unsafe`, we promise the compiler: "I have verified this specific hardware interaction acts the way I expect."

## 3. The Editor Core (`src/editor/mod.rs`)

The logic resides in `src/editor/mod.rs`. It manages the text state and handles user interaction.

### A. Data Structure
We use a `Vec<Vec<u8>>` to represent the text buffer.
*   **Outer `Vec`**: Represents the list of rows (lines).
*   **Inner `Vec<u8>`**: Represents the characters in a single line.

This structure allows easy insertion and line splitting without managing a massive contiguous array.

**Core Logic:**
1.  **Input Loop**: An async task consumes scancodes from the keyboard driver.
2.  **Action Dispatch**:
    *   **Unicode Characters**: Inserted into the buffer at the current cursor position.
    *   **Enter (\n)**: Splits the current line into two.
    *   **Backspace**: Removes character or merges lines if at start of line.
    *   **Arrows**: Updates `cursor_row` / `cursor_col`.
3.  **Rendering**: After every keypress, the `draw_full_screen` function redraws the visible portion of the buffer.

**Code Snippet (Main Loop):**
```rust
pub async fn run(mut self) {
    self.draw_full_screen();
    
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(ScancodeSet1::new(), layouts::Us104Key, HandleControl::Ignore);

    while let Some(scancode) = scancodes.next().await {
        // This loop only wakes up when hardware interrupts verify a keystroke
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                // ... Handle key ...
            self.draw_full_screen();
                self.update_cursor();
            }
        }
    }
}
```

### C. OS Integration (`src/main.rs`)
We modified `main.rs` to spawn the `Editor` task immediately after initialization.

```rust
let mut executor = Executor::new();
let editor = os::editor::Editor::new();
executor.spawn(Task::new(editor.run()));
executor.run(); // Starts the OS event loop
```

## 3. Importance of Design Choices
*   **No Filesystem**: Since we don't have a disk driver yet, the editor is "in-memory". This simplifies the implementation to focus on UI and input handling first.
*   **Async/Await**: Using Rust's async system allows the editor to sleep when not processing keys, saving CPU resources compared to a busy loop.
*   **Direct VGA Writing**: Bypassing the rolling console log allows for a stable "TUI" (Text User Interface) feel, similar to `nano` or `vim`, rather than a command line stream.

## 4. Optimization: Flicker-Free Rendering

### The Problem
Initially, the `draw_full_screen` method would call `writer.clear_screen()` followed by writing all the characters. This caused a noticeable flicker on every keystroke because the screen was briefly blank (black) before the text was redrawn.

### The Solution: Overwrite Strategy
We optimized the rendering loop to **never clear the screen**. Instead, we:
1.  **Lock Once**: We acquire the `WRITER` lock once for the entire frame to prevent contention and ensure atomic updates.
2.  **Overwrite Content**: We write the character that should be there.
3.  **Overwrite Empty Space**: If a line is shorter than the screen width, or if we are past the end of the file, we explicitly write spaces (`b' '`) to "clear" the old content that might have been there.

This ensures every pixel on the screen transitions directly from "Old State" -> "New State" without an intermediate "Blank State", eliminating flicker.

**Optimized Code Snippet:**
```rust
x86_64::instructions::interrupts::without_interrupts(|| {
    let mut writer = WRITER.lock();
    
    // ... Draw Header ...

    for val_row in 0..self.screen_rows {
        // ... Determine content ...
        
        for val_col in 0..self.screen_cols {
            let char_code = if file_col < len {
                row_content[file_col]
            } else {
                b' ' // Critical: Write space to clear old text without clearing screen
            };
            
            writer.write_char_at(screen_row, val_col, char_code, ColorCode::new(Color::White, Color::Black));
        }
    }
});
```
