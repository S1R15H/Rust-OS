#### **1. What are Hardware Interrupts?**

- **CPU Exceptions (like `int3` or Page Faults)** are _synchronous_. They are triggered directly by the CPU's execution of code.
- **Hardware Interrupts** are _asynchronous_. They are sent by external hardware at any time, interrupting the CPU's current work.
- To manage many hardware devices, the CPU uses a chip called the **Programmable Interrupt Controller (PIC)**. This chip gathers all interrupt signals and forwards them to the CPU one by one.

---

#### **2. The Problem: The 8259 PIC**

- Modern systems have a newer controller (APIC), but the (very old) **8259 PIC** is still enabled by default for backward compatibility. We must interact with this old chip.
- **The Problem:** By default, the 8259 PIC sends hardware interrupts to the CPU using vectors **0–15**.
- This is a critical conflict! These vectors are already used by our **CPU exceptions** (e.g., #0 is "Divide by Zero", #8 is "Double Fault").
- If a hardware timer "ticks" (vector #0) at the same time as a "divide by zero" error (vector #0), the CPU won't know which one happened.

---

#### **3. The Solution: Remapping the PIC**

- We must "remap" the PIC, which means re-programming it to use a different, non-conflicting range of vectors.
- **New Range:** We will map the PIC interrupts to vectors **32–47**. This range is free and designated for user-defined interrupts.
- **Implementation:** We don't do this manually. We add the `pic8259` crate, which provides a safe wrapper to handle this complex and `unsafe` remapping process.

---

#### **4. Implementation Steps**

**Step 1: Add the `pic8259` Crate**

- This crate provides the `ChainedPics` struct, which manages the two PIC chips (master and slave) in a modern system.
- We create a global, static, and locked instance:
    ```rust
    use spin::Mutex;
    use pic8259::ChainedPics;
    
    pub static PICS: Mutex<ChainedPics> =
        Mutex::new(unsafe { ChainedPics::new(32, 40) }); // (Offset for master, Offset for slave)
    ```
    
- `32` means the master PIC's interrupts will start at vector 32.
- `40` means the slave PIC's interrupts will start at vector 40.

**Step 2: Initialize the PIC**

- In our `init` function, we must call the PIC's `initialize()` method. This is an `unsafe` operation (which the crate handles) that sends the remapping commands to the PIC hardware.

**Step 3: Add Handlers to the IDT**

- We add a handler for the hardware timer (which is now at vector **32**) to our `InterruptDescriptorTable` (IDT).
    ```rust
    // In IDT setup...
    idt[InterruptIndex::Timer.as_usize()]
        .set_handler_fn(timer_interrupt_handler);
    ```
    

**Step 4: Enable Interrupts**

- The CPU starts with interrupts disabled (it ignores the PIC).
- To "turn on" interrupts, we execute the `sti` ("set interrupts") instruction. The `x86_64` crate provides a safe function for this.
    ```rust
    x86_64::instructions::interrupts::enable(); // Executes the `sti` instruction
    ```
- Once this runs, the CPU will _immediately_ start listening for interrupts from the PIC.

---

#### **5. The "End of Interrupt" (EOI) Signal**

- This is the most critical part of handling hardware interrupts.
- When the PIC sends an interrupt (e.g., a timer tick), it **pauses**. It will _not_ send any more timer interrupts until the CPU tells it that the interrupt has been handled.
- **Problem:** If we _forget_ to send this "all clear" signal, the timer will tick **once**, our handler will run, and then the timer will be silent forever, freezing our system clock.
- **Solution:** At the _end_ of our interrupt handler, we **must** send an **"End of Interrupt" (EOI)** signal back to the PIC.
    
- **Implementation:**
    ```rust
    extern "x86-interrupt" fn timer_interrupt_handler(
        _stack_frame: InterruptStackFrame)
    {
        serial_print!("."); // Show that the interrupt happened
    
        // Send the EOI signal
        unsafe {
            PICS.lock()
                .notify_end_of_interrupt(InterruptIndex::Timer.as_numeric_value());
        }
    }
    ```
- When we run the kernel now, we see a constant stream of `.` characters on the console, proving that the timer is successfully firing interrupts, our handler is running, and the EDIs are being sent correctly.

---
#### **6. The New Problem: Deadlocks**

- Our `timer_interrupt_handler` calls `PICS.lock()`.
- What happens if the interrupt _occurs_ while our main `_start` function is _also_ holding the `PICS.lock()` (or any other `Mutex`)?
    1. Main code (e.g., `_start`) locks the `PICS` Mutex.
    2. The hardware timer "ticks," triggering an interrupt.
    3. The CPU _pauses_ `_start` and jumps to `timer_interrupt_handler`.
    4. The handler tries to call `PICS.lock()`, but the lock is **already held** by the paused `_start` function.
    5. The handler "spins," waiting for the lock to be released...
    6. ...but `_start` (which holds the lock) can't resume _until the handler finishes_.
- This is a **deadlock**. The system freezes completely.

---

#### **7. The Solution: Disabling Interrupts**

- We must guarantee that an interrupt cannot occur when we are holding a lock that _the interrupt handler also needs_.
- **Solution:** We wrap any critical code (like `PICS.lock().initialize()`) in a special block that temporarily disables interrupts.
    ```rust
    use x86_64::instructions::interrupts;
    
    interrupts::without_interrupts(|| {
        // All code in this closure runs with interrupts disabled.
        // No interrupt can happen here, so no deadlock is possible.
        PICS.lock().initialize();
    });
    
    // Interrupts are automatically re-enabled here.
    ```

---

#### **8. Example 2: Keyboard Input**

This section uses all the concepts above to build a keyboard driver.

1. **Add IDT Entry:** We add a `keyboard_interrupt_handler` to the IDT at vector **33** (the PIC's second interrupt line).
2. **Read Scancode:** The handler (which must also send an EOI) reads the "scancode" (a raw byte) from the keyboard's I/O port (`0x60`).
    ```rust
    use x86_64::instructions::port::Port;
    
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    ```
1. **Translate Scancode:** The scancode (e.g., `0x1E`) is not the character (`'a'`). We add the `pc-keyboard` crate to translate these scancodes into letters, numbers, and key events (like "key pressed" or "key released").
2. **Print to Screen:** The handler uses a global `Mutex<Keyboard>` (from the `pc-keyboard` crate) to process the scancode, which returns an `Option<DecodedKey>`. We then print this key to the VGA buffer.

*This final example demonstrates the complete interrupt handling loop: **Hardware Event (key press) -> PIC -> CPU -> IDT -> Our Handler -> Read Port -> Send EOI -> Translate Scancode -> Print to Screen.**