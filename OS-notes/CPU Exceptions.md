
#### **1. What Are CPU Exceptions?**

- An exception is like a hardware-level error. It's triggered directly by the CPU when an illegal operation occurs.
- **Examples:**
    - **Divide by Zero:** The CPU tried to divide a number by 0.
    - **Page Fault:** The program tried to access a piece of memory that isn't mapped or isn't allowed.
    - **Breakpoint:** A special exception (vector #3) that is _not_ an error. It's triggered by the `int3` instruction and is used by debuggers to "pause" a program.

---

#### **2. The Interrupt Descriptor Table (IDT)**

- **Problem:** How does the CPU know _which function to call_ for a "divide by zero" error vs. a "page fault"?
- **Solution:** The CPU uses the **Interrupt Descriptor Table (IDT)**.
- The IDT is a table (an array) that you set up. Each entry in the table corresponds to an exception number (called a "vector").
- You program the IDT to tell the CPU:
    - For exception **#0** (Divide by Zero), call my `divide_by_zero_handler` function.
    - For exception **#3** (Breakpoint), call my `breakpoint_handler` function.
    - For exception **#14** (Page Fault), call my `page_fault_handler` function.

---

#### **3. Setting up the IDT in Rust**

**Step 1: Create Handler Functions**

- We create a new `interrupts.rs` module.
- We use the `x86-interrupt` calling convention, which is a new, safe way to create handler functions. This ABI automatically saves all CPU registers when the exception occurs and restores them when it's done.
- **Example Handler:**
    ```rust
    // The `extern "x86-interrupt"` makes this a safe interrupt handler
    extern "x86-interrupt" fn breakpoint_handler(
        stack_frame: InterruptStackFrame)
    {
        serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
    }
    ```
    

**Step 2: Create and Load the IDT**

- We use the `InterruptDescriptorTable` struct from the `x86_64` crate.
- We create a global, static IDT using `lazy_static`.
- We "program" the IDT by setting its entries to point to our handlers.
    ```rust
    lazy_static! {
        static ref IDT: InterruptDescriptorTable = {
            let mut idt = InterruptDescriptorTable::new();
    
            // Set the handler for vector #3 (breakpoint)
            idt.breakpoint.set_handler_fn(breakpoint_handler);
    
            // ... set other handlers ...
    
            idt
        };
    }
    ```
- We create an `init()` function that loads this IDT into the CPU's `IDTR` register using the `lidt` (Load IDT) instruction. We call this `init()` function from our `_start` function.
    

**Step 3: Test It**

- After loading the IDT, we can just call `x86_64::instructions::interrupts::int3();` in our `_start` function.
- This triggers a breakpoint. The CPU will stop, look in the IDT at entry #3, find our `breakpoint_handler`, and jump to it. Our handler will print the exception info to the serial port, and then the CPU will resume right after the `int3` call.

---

#### **4. The Double Fault Problem (and the GDT)**

- **Critical Problem:** What happens if an exception occurs _while the CPU is already handling another exception_? This is a **Double Fault**. (Example: A stack overflow happens _inside_ the breakpoint handler).
- A double fault is exception **#8**. We must create a handler for it.
- **Bigger Problem:** If the double fault handler _also_ fails (e.g., because the kernel stack is completely corrupt), the CPU gives up, triggers an uncatchable **Triple Fault**, and reboots. This is the ultimate kernel crash.
- **Solution:** We must make our double fault handler _impossible_ to fail. We do this by giving it its own, separate, brand-new stack.
- **The Global Descriptor Table (GDT):** We use the GDT (a much older table than the IDT) to tell the CPU, "For a regular exception, use the main kernel stack. But for a double fault, _immediately switch to this other, safe stack_ before you even run the handler."
- This ensures that even if the main kernel stack is corrupt, the double fault handler can run on its "emergency stack," log the error, and prevent a triple fault.