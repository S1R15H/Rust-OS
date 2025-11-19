### **Design Goals**

- **Correctness:** The allocator must _never_ give out a block of memory that is already in use.
- **Performance:** It should be fast and efficient.
- **Memory Use:** It should use memory effectively and avoid fragmentation (wasting space).
- **Concurrency:** It should ideally work safely if multiple CPU cores try to access it at once.

---

### **Allocator 1: Bump Allocator**

This is the simplest possible allocator.

- **Idea:** It's like a roll of tape. It keeps a pointer (`next`) to the start of the unused memory.
- **`alloc`:** When you request memory, it "bumps" the pointer, giving you the memory block it just passed over. It's extremely fast because it's just moving a pointer.
- **`dealloc`:** This is the critical weakness. A bump allocator **cannot free individual blocks**.
- **The Workaround:** It keeps a _counter_ of active allocations. When `dealloc` is called, it just decrements the counter. When the counter reaches zero (meaning _all_ allocations have been freed), it resets the `next` pointer back to the start of the heap, making all the memory available again.
- **Result:**
    - **Pros:** Incredibly fast allocation.
    - **Cons:** Useless for general-purpose use because you can't free individual blocks. It leads to quickly running out of memory.

---

### **Allocator 2: Linked List Allocator**

This is a much more capable design (and the one used by the `linked_list_allocator` crate in the previous post).

- **Idea:** It creates a "free list" — a linked list of all the free memory blocks (or "regions").
- **`alloc`:** When you request memory (e.g., 8 bytes), it walks the linked list looking for a free block that is _at least_ 8 bytes large.
    - If it finds a perfect 8-byte block, it removes it from the list and gives it to you.
    - If it finds a larger block (e.g., 32 bytes), it **splits** the block. It gives you 8 bytes and puts the remaining 24-byte block back into the free list.
- **`dealloc`:** When you free a block, it simply adds that block back to the front of the free list.
- **Key Trick:** Where is the linked list's "next" pointer stored? **Inside the free memory block itself!** Since the block is free, it's just sitting there, so we can use its first 8 bytes to store a pointer to the _next_ free block.
    
    - **Result:**
    - **Pros:** Can free memory. Solves the bump allocator's main problem.
    - **Cons:** **Slow.** Both `alloc` and `dealloc` can be O(n) operations because the allocator might have to walk a long list to find a suitable block. It can also suffer from **external fragmentation** (the free list might be full of tiny blocks, but none large enough for your request).

---

### **Allocator 3: Fixed-Size Block Allocator**

This is a more advanced, high-performance design that improves on the linked list.

- **Idea:** It recognizes that most allocations are for a few common sizes (e.g., 8, 16, 32, 64 bytes...). Instead of _one_ free list, it creates _multiple_ free lists, one for each "block size."
- It has an array of free lists:
    
    - `list_heads[0]` -> Head of the 8-byte free list.
    - `list_heads[1]` -> Head of the 16-byte free list.
    - ...
    - `list_heads[8]` -> Head of the 2048-byte free list.
- **`alloc`:**
    1. It checks your requested size (e.g., 20 bytes).
    2. It rounds _up_ to the nearest block size (e.g., 32 bytes).
    3. It goes _directly_ to the 32-byte free list and just pops the first block off. This is an **O(1) operation — extremely fast.**
- **`dealloc`:**
    1. It checks the freed block's size (e.g., 32 bytes).
    2. It goes _directly_ to the 32-byte list and adds the block to the front. This is also an **O(1) operation.**
- **What about large/weird sizes?** For any allocation that's too big (e.g., > 2048 bytes), it just uses a **fallback allocator** (like the linked list allocator) to handle it.

- **Result:**
    
    - **Pros:** Extremely fast (O(1)) for common allocations. Avoids fragmentation better than the linked list.
    - **Cons:** Can waste a little memory due to rounding up (requesting 20 bytes uses a 32-byte block). This is called **internal fragmentation**, but it's generally considered a good trade-off.