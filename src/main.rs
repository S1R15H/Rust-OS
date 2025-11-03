#![no_std]
#![no_main]
/*
Building my own Operating System for fun
Objective: Learn and explore low-level programming concepts and OS development
*/

/// Creating a panic handler function for no_std environment
use core::panic::PanicInfo;

/// This function is called on panic.
/// PanicInfo contains the file and line where panic happened.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

static HELLO: &[u8] = b"Hello World!";

#[unsafe(no_mangle)] // Disable name mangling to ensure the function name is preserved
pub extern "C" fn _start() -> ! {
    // this function is the entry point, since the linker looks for a function
    // named `_start` by default
    let vga_buffer = 0xb8000 as *mut u8;

    for (i, &byte) in HELLO.iter().enumerate() {
        unsafe {
            *vga_buffer.offset(i as isize * 2) = byte;
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }
    loop{}
}
