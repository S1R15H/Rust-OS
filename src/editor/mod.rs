use alloc::vec::Vec;
use alloc::vec;
use crate::vga_buffer::{WRITER, ColorCode, Color};
use pc_keyboard::{DecodedKey, Keyboard, ScancodeSet1, layouts, HandleControl};
use futures_util::stream::StreamExt;
use crate::task::keyboard::ScancodeStream;
use crate::{exit_qemu, QemuExitCode};

pub struct Editor {
    buffer: Vec<Vec<u8>>,
    cursor_row: usize, // Row in the text buffer
    cursor_col: usize, // Column in the text buffer
    row_offset: usize, // Scroll offset
    col_offset: usize,
    screen_rows: usize,
    screen_cols: usize,
}

impl Editor {
    pub fn new() -> Self {
        Editor {
            buffer: vec![Vec::new()],
            cursor_row: 0,
            cursor_col: 0,
            row_offset: 0,
            col_offset: 0,
            screen_rows: 12, // 25 total - 12 header - 1 footer = 12
            screen_cols: 80,
        }
    }

    pub async fn run(mut self) {
        self.draw_full_screen();
        self.update_cursor(); // Fix: Ensure cursor is placed correctly at startup
        
        // Initialize keyboard scancode stream
        let mut scancodes = ScancodeStream::new();
        let mut keyboard = Keyboard::new(ScancodeSet1::new(), layouts::Us104Key, HandleControl::Ignore);

        while let Some(scancode) = scancodes.next().await {
            if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
                if let Some(key) = keyboard.process_keyevent(key_event) {
                    match key {
                        DecodedKey::Unicode(c) => self.insert_char(c),
                        DecodedKey::RawKey(key) => self.process_raw_key(key),
                    }
                    self.scroll();
                    self.draw_full_screen(); // Naive redraw per keypress
                    self.update_cursor();
                }
            }
        }
    }

    fn insert_char(&mut self, c: char) {
        if c == '\n' {
            self.insert_newline();
        } else if c == '\x08' || c == '\x7f' {
            // Handle Backspace/Delete if they come as Unicode
            self.process_raw_key(pc_keyboard::KeyCode::Backspace);
        } else if c == '\x1b' {
            // Handle Escape if it comes as Unicode
            self.process_raw_key(pc_keyboard::KeyCode::Escape);
        } else if c.is_ascii() && !c.is_control() {
            // Check if we are at the end of the line
            if self.cursor_col > self.buffer[self.cursor_row].len() {
                 self.cursor_col = self.buffer[self.cursor_row].len();
            }
            let byte = c as u8;
            self.buffer[self.cursor_row].insert(self.cursor_col, byte);
            self.cursor_col += 1;
        }
    }

    fn insert_newline(&mut self) {
        let remainder = if self.cursor_col < self.buffer[self.cursor_row].len() {
            self.buffer[self.cursor_row].split_off(self.cursor_col)
        } else {
            Vec::new()
        };
        self.buffer.insert(self.cursor_row + 1, remainder);
        self.cursor_row += 1;
        self.cursor_col = 0;
    }

    pub fn process_raw_key(&mut self, key: pc_keyboard::KeyCode) {
        use pc_keyboard::KeyCode;
        match key {

            KeyCode::ArrowUp => {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                }
            }
            KeyCode::ArrowDown => {
                if self.cursor_row < self.buffer.len() - 1 {
                    self.cursor_row += 1;
                }
            }
            KeyCode::ArrowLeft => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.buffer[self.cursor_row].len();
                }
            }
            KeyCode::ArrowRight => {
                if self.cursor_col < self.buffer[self.cursor_row].len() {
                    self.cursor_col += 1;
                } else if self.cursor_row < self.buffer.len() - 1 {
                    self.cursor_row += 1;
                    self.cursor_col = 0;
                }
            }
            KeyCode::Backspace | KeyCode::Delete => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                    self.buffer[self.cursor_row].remove(self.cursor_col);
                } else if self.cursor_row > 0 {
                    let mut current_line = self.buffer.remove(self.cursor_row);
                    self.cursor_row -= 1;
                    self.cursor_col = self.buffer[self.cursor_row].len();
                    self.buffer[self.cursor_row].append(&mut current_line);
                }
            }
            KeyCode::Escape => {
                exit_qemu(QemuExitCode::Success);
            }
            _ => {}
        }
        
        // Clamp cursor col if we moved vertically to a shorter line
        if self.cursor_col > self.buffer[self.cursor_row].len() {
            self.cursor_col = self.buffer[self.cursor_row].len();
        }
    }
    
    fn scroll(&mut self) {
        if self.cursor_row < self.row_offset {
            self.row_offset = self.cursor_row;
        }
        if self.cursor_row >= self.row_offset + self.screen_rows {
            self.row_offset = self.cursor_row - self.screen_rows + 1;
        }
        if self.cursor_col < self.col_offset {
            self.col_offset = self.cursor_col;
        }
        // Simple horizontal scroll
        if self.cursor_col >= self.col_offset + self.screen_cols {
            self.col_offset = self.cursor_col - self.screen_cols + 1;
        }
    }

    fn draw_full_screen(&self) {
        x86_64::instructions::interrupts::without_interrupts(|| {
            let mut writer = WRITER.lock();
            let header_bg = ColorCode::new(Color::White, Color::Black); // Text color for ASCII art
            let info_color = ColorCode::new(Color::LightGray, Color::Black);

            // 1. Draw ASCII Art Header (approx 6 lines)
            // RUST OS
            let art = [
                  "       ____             __  ____  _____  ",
                r#"      / __ \__  _______/ /_/ __ \/ ___/  "#,
                r#"     / /_/ / / / / ___/ __/ / / /\__ \   "#,
                r#"    / _, _/ /_/ (__  ) /_/ /_/ /___/ /  "#,
                r#"   /_/ |_|\__,_/____/\__/\____//____/   "#,
                "############################################",
            ];

            for (i, line) in art.iter().enumerate() {
                // Clear full line first
                for col in 0..80 {
                     writer.write_char_at(i, col, b' ', header_bg);
                }
                // Write art centered-ish or left
                for (j, byte) in line.bytes().enumerate() {
                    let color = if byte != b' ' { 
                         ColorCode::new(Color::LightCyan, Color::Black) 
                    } else { 
                        header_bg 
                    };
                    writer.write_char_at(i, j + 2, byte, color);
                }
            }

            // 2. Maximum System Info
            let info_lines = [
                "Welcome to my custom Rust OS.",
                "Memory: Paging (4-level) & Heap (Linked List Allocator)",
                "Process Management: Cooperative Multitasking (Async/Await Executor)",
                "Exception Handling: IDT with Double Fault stack switching",
                "Try typing below! This is a simple memory-resident editor.",
                "--------------------------------------------------------------------------------",
            ];
            
            let start_row = art.len(); // 6
            for (i, line) in info_lines.iter().enumerate() {
                let row = start_row + i;
                 for col in 0..80 {
                     writer.write_char_at(row, col, b' ', info_color);
                }
                for (j, byte) in line.bytes().enumerate() {
                    writer.write_char_at(row, j + 2, byte, info_color);
                }
            }

            // 3. Draw Editor Content
            let header_height = 12; 
            for val_row in 0..self.screen_rows {
                let file_row = self.row_offset + val_row;
                // Add header height offset
                let screen_row = val_row + header_height; 

                if file_row >= self.buffer.len() {
                    writer.write_char_at(screen_row, 0, b'~', ColorCode::new(Color::LightBlue, Color::Black));
                    for col in 1..80 {
                        writer.write_char_at(screen_row, col, b' ', ColorCode::new(Color::White, Color::Black));
                    }
                    continue;
                }

                let row_content = &self.buffer[file_row];
                let len = row_content.len();

                for val_col in 0..self.screen_cols {
                    let file_col = self.col_offset + val_col;
                    let char_code = if file_col < len {
                        row_content[file_col]
                    } else {
                        b' ' 
                    };
                    writer.write_char_at(screen_row, val_col, char_code, ColorCode::new(Color::White, Color::Black));
                }
            }

            // 4. Draw Footer
            let footer_row = 24;
            let footer_bg = ColorCode::new(Color::Black, Color::LightGray);
            let footer_text = " [ESC] Shutdown System ";
            // Clear footer line
            for col in 0..80 {
                writer.write_char_at(footer_row, col, b' ', footer_bg);
            }
            // Write text
            for (i, byte) in footer_text.bytes().enumerate() {
                writer.write_char_at(footer_row, i, byte, footer_bg);
            }
        });
    }

    fn update_cursor(&self) {
        let header_height = 12;
        let screen_row = self.cursor_row - self.row_offset + header_height; 
        let screen_col = self.cursor_col - self.col_offset;
        
        x86_64::instructions::interrupts::without_interrupts(|| {
            WRITER.lock().set_cursor_position(screen_row, screen_col);
        });
    }
}
