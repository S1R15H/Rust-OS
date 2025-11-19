
#### **1. The Core Problem: Virtual vs. Physical Addresses**

- **The Conflict:** The CPU's `CR3` register stores the **physical address** of the Level 4 page table. All page table entries also store **physical addresses**.
- However, our kernel is running _with paging enabled_, which means it can only see **virtual addresses**. It cannot directly access a physical address like `0x1000`.
- **The Question:** How can the kernel read and write to the page tables if it can't access their physical addresses?

#### **2. The Solution: Physical Memory Offset**

We need a way to reliably access any physical address from within our kernel's virtual address space.

- **The `bootloader` Crate:** The bootloader solves this for us! It maps the _entire_ physical memory to a high, unused range of the virtual address space.
- **How it works:** It creates a **physical memory offset**. For example, it maps all of physical memory to start at the virtual address `0xFFFF800000000000`.
- **The Result:**
    - To access physical address `0x1000`, the kernel just accesses virtual address `0xFFFF800000001000`.
    - To access physical address `0xB8000` (VGA buffer), the kernel can use `0xFFFF8000000B8000`.
- This single mapping gives the kernel a "back door" to read and write _all_ physical memory, including the page tables, just by using a known virtual address offset.

---

#### **3. Step 1: Translating Addresses (Reading the Tables)**

Now that we _can_ access the tables, we can write a function to "walk" them and see how they are mapped.

- We create a `translate_addr` function that takes a **virtual address**.
- **The Process:**
    1. It reads the `CR3` register to get the physical address of the L4 table.
    2. It uses the **physical memory offset** to convert this to a virtual address the kernel can read.
    3. It "walks" the 4-level tables: It reads the entry from the L4 table, which points to the L3 table. It reads the L3 table, which points to the L2, and so on.
    4. Finally, the L1 table entry gives it the **physical frame** that the virtual address is mapped to.
- This proves we can successfully _read_ the page tables.

---

#### **4. Step 2: Creating Mappings (Writing to the Tables)**

This is the main goal. We need to map a new, _unused_ virtual page to a new, _unused_ physical frame.

**Requirement 1: A Frame Allocator**
- We can't just "pick" a physical frame; we might overwrite something. We need a system to manage which frames are free.
- The bootloader gives us a **memory map** (a list of all memory and whether it's in use).
- We create a `BootInfoFrameAllocator` that reads this map and hands out free, unused physical frames one by one.

**Requirement 2: The `create_mapping` Function**

- This function takes a virtual page (e.g., `0x1234000`) and maps it.
- **The Process:**
    1. It asks the `BootInfoFrameAllocator` for a free physical frame.
    2. It "walks" the 4-level page tables for the virtual address `0x1234000`.
    3. **Crucially:** If the intermediate tables (e.g., L3 or L2) don't exist yet, it asks the frame allocator for _more_ free frames to _create_ those tables.
    4. It finally writes the new mapping into the L1 table: "Virtual Page `0x1234` now points to Physical Frame `[newly allocated frame]`."

**Requirement 3: Flushing the TLB**

- **The Problem:** The CPU has a cache called the **TLB (Translation Lookaside Buffer)** that remembers recent address translations.
- If we change the page tables, the CPU might _not notice_ and keep using the _old, cached_ translation, leading to a crash.
- **The Solution:** After creating a new mapping, we _must_ tell the CPU to flush its cache for that page. We use the `invlpg` ("invalidate page") instruction. This forces the CPU to re-read our new mapping.

#### **5. The Result**

The article ends by creating a new `map_to` function, mapping a page, writing a value to it, and confirming the write was successful. This proves the kernel has full control over the paging system.