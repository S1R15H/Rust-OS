#### **1. What is a Double Fault?**

A double fault (exception #8) happens when the CPU tries to handle an exception but can't.

- **Example:** You access an unmapped memory address (`0xdeadbeef`).
    1. This triggers a **Page Fault** (exception #14).
    2. The CPU tries to find the Page Fault handler in your Interrupt Descriptor Table (IDT).
    3. If you haven't defined a Page Fault handler, the CPU can't proceed.
    4. This failure to handle the first exception triggers a **Double Fault**.

If you also don't have a double fault handler, the CPU fails again, triggering a **Triple Fault** and a system reset (boot loop).

---

#### **2. The Problem: Kernel Stack Overflow**

Just adding a double fault handler to the IDT isn't enough. Consider this scenario:
1. Your kernel code has an infinite recursion, causing a **stack overflow**.
2. The stack pointer goes past the end of the stack and hits a "guard page" (an unmapped page).
3. This triggers a **Page Fault**.
4. The CPU tries to handle the Page Fault. To do this, it must push the exception details (the `InterruptStackFrame`) _onto the stack_.
5. ...but the stack is _still_ pointing to the bad guard page!
6. This causes a _second_ **Page Fault** while trying to handle the _first_ Page Fault.
7. As per the CPU rules, a Page Fault followed by another Page Fault triggers a **Double Fault**.
8. The CPU now tries to handle the Double Fault. To do this, it must push the double fault's stack frame _onto the stack_.
9. ...but the stack is _still_ pointing to the bad guard page!
10. This causes a _third_ **Page Fault**, which triggers a **Triple Fault**. The system reboots.

The core problem is that the stack is unusable, and the CPU needs a valid stack to handle any exception.

---

#### **3. The Solution: Switching Stacks**

To solve this, we must give the CPU a _separate, known-good stack_ to use _only_ for double fault emergencies.
This is a three-step process involving the GDT, TSS, and IDT.
**Step 1: Create the Task State Segment (TSS)**
- The **TSS** is a legacy structure that is still used on x86_64 to hold special tables.
- One of these tables is the **Interrupt Stack Table (IST)**.
- The **IST** is a list of 7 pointers to 7 separate "emergency" stacks.
- **Action:** We create a new, static `TaskStateSegment` (TSS). We also create a new, static array (e.g., `[u8; 4096]`) to serve as our emergency stack. We set the 0th entry in the `interrupt_stack_table` of our TSS to point to the top of this new stack.

**Step 2: Create the Global Descriptor Table (GDT)**
- The CPU doesn't load the TSS directly. We must use another legacy structure, the **GDT**, to tell the CPU about our TSS.
- **Action:** We create a new `GlobalDescriptorTable` (GDT) using `lazy_static`. We add two entries to it: a required "kernel code segment" and, most importantly, a `Descriptor::tss_segment(&TSS)` that points to our new TSS.
- We then write a `gdt::init()` function that loads this GDT into the CPU (using `GDT.load()`) and tells the CPU to use our TSS (using `x86_64::instructions::tables::load_tss()`).

**Step 3: Update the Interrupt Descriptor Table (IDT)**
- This is the final step that ties it all together.
- **Action:** In our `interrupts.rs` file, when we set up our `double_fault_handler`, we also set its **stack index**.
    ```rust
    lazy_static! {
        static ref IDT: InterruptDescriptorTable = {
            let mut idt = InterruptDescriptorTable::new();
            // ...
    
            // Set the double fault handler
            unsafe {
                idt.double_fault.set_handler_fn(double_fault_handler)
                    .set_stack_index(our_gdt::DOUBLE_FAULT_IST_INDEX); // <-- THIS IS THE MAGIC
            }
    
            idt
        };
    }
    ```
- This one line tells the CPU: "Before you even _try_ to run the `double_fault_handler`, _immediately_ switch to the stack found at index 0 of the Interrupt Stack Table."

---

#### **4. The Result**

Now, when the stack overflow happens:
1. Page Fault -> Page Fault -> **Double Fault** is triggered.
2. The CPU sees it's a double fault and checks the IDT.
3. The IDT entry tells it to **use stack #0 from the IST**.
4. The CPU **immediately switches** to our new, safe, empty stack.
5. _Now_ on this new stack, the CPU safely pushes the double fault's stack frame.
6. The CPU calls our `double_fault_handler`, which runs on the new stack, logs the error, and prevents a triple fault.