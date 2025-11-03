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
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[unsafe(no_mangle)] // Disable name mangling to ensure the function name is preserved
pub extern "C" fn _start() -> ! {
    // this function is the entry point, since the linker looks for a function
    // named `_start` by default
    println!("Hello World{}", "!");
    loop{}
}

mod vga_buffer;
