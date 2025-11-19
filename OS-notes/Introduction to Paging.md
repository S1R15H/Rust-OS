#### **1. The Problem: Memory Protection & Fragmentation**

- **The Goal:** An OS must isolate programs from each other (a browser shouldn't crash a text editor).
- **Old Method (Segmentation):** This method defines large, variable-sized memory regions ("segments").
- **The Problem with Segmentation:** It leads to **external fragmentation**. You might have 100MB of free RAM, but if it's in 10MB chunks, you can't start a program that needs a single 50MB block. This wastes space.

---

#### **2. The Solution: Paging**

- Paging avoids fragmentation by dividing all memory into small, **fixed-size blocks**.
- **Pages:** A block of **virtual memory** (the addresses the program sees).
- **Frames:** A block of **physical memory** (the actual RAM chips).
- **The Core Idea:** Paging "translates" virtual addresses into physical addresses. This allows a program's memory to be scattered all over physical RAM in a non-continuous way, which completely solves the external fragmentation problem.

---

#### **3. Page Tables: The "Address Book"**

- **Problem:** How does the CPU know which virtual page maps to which physical frame?
- **Solution:** The OS maintains a **page table** for each program. This is a lookup table (an "address book") that stores the mapping.
- **How it works:**
    1. The program tries to access a **virtual address**.
    2. The CPU (specifically, the **MMU** - Memory Management Unit) hardware automatically looks in the _current program's page table_.
    3. The table "translates" the virtual page into a physical frame address.
    4. The CPU accesses the correct data in physical RAM.
- The `CR3` register on the CPU is a special register that stores the physical memory address of the _currently active page table_.

---

#### **4. The Problem with Large Address Spaces**

- A 64-bit address space is _enormous_.
- If you had one giant page table to map every possible 4KiB page, the page table itself would be **impossibly large** (billions of entries, taking up terabytes of RAM).

---

#### **5. The Solution: Multi-Level Page Tables**

- Instead of one giant table, we use a "tree" of tables.
- **The x86_64 architecture uses a 4-level page table.**
- **How it works:**
    1. The `CR3` register points to the **Level 4 Table**.
    2. An entry in the L4 table points to a **Level 3 Table**.
    3. An entry in the L3 table points to a **Level 2 Table**.
    4. An entry in the L2 table points to a **Level 1 Table**.
    5. An entry in the L1 table _finally_ points to the **physical frame** containing the data.
- **The Benefit:** If a program only uses a tiny bit of memory, we only need a few small tables. We don't have to create tables for the vast, unused portions of the 64-bit address space, saving a massive amount of memory.

---

#### **6. Key Paging Concepts**

- **TLB (Translation Lookaside Buffer):** Walking a 4-level table is slow. The CPU has a small, super-fast cache called the **TLB** that remembers recent translations. This makes most memory accesses very fast.
- **Page Fault:** This is a CPU exception (like the ones from the previous article). It happens when the CPU tries to translate an address but finds an invalid entry, such as:
    - The page isn't mapped (it doesn't exist).
    - The page is "read-only," but the code tried to _write_ to it.
- The **`CR2` register** is a special register that, after a page fault, holds the virtual address that _caused_ the fault.
- **Your kernel is already on paging:** The bootloader set up a 4-level page table for you. This is why accessing random memory (like `0xdeadbeaf`) causes a _page fault_ (which triggers your double fault handler) instead of corrupting physical RAM.