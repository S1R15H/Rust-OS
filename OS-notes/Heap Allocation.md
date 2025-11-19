#### **1. Why We Need a Heap**

- **Local Variables (on the stack):** Are very fast, but they are temporary. They are destroyed as soon as their function exits.
- **Static Variables:** Live for the entire program, but their size must be known at compile time.
- **Heap Memory:** Provides the solution for data that needs to live longer than a single function _but_ not for the entire program, or for data that needs to grow in size (like a `Vec`). The heap allows us to request and free memory at any time.

---

#### **2. The `alloc` Crate**

- Rust's heap allocation types (`Box`, `Vec`, etc.) live in the `alloc` crate.
- Like the `core` crate, `alloc` is a subset of the standard library that doesn't depend on an OS.
- **The Catch:** To use the `alloc` crate, you must provide a **global allocator**—a mechanism that `alloc` can call to actually get and free memory.

---

#### **3. The `GlobalAlloc` Trait**

- To provide a global allocator, you must create a `static` item with the `#[global_allocator]` attribute.
- This item must implement the `GlobalAlloc` trait, which mainly requires two `unsafe` functions:
    - `alloc(layout: Layout) -> *mut u8`: Takes a requested size/alignment and returns a raw pointer to a block of memory.
    - `dealloc(ptr: *mut u8, layout: Layout)`: Takes a pointer (from `alloc`) and frees that memory.

---

#### **4. Step 1: Defining the Heap's Virtual Memory**

- We can't just give the allocator random memory. First, we must "set aside" a virtual memory region for the heap.
- **Action:** We define a constant for the heap's start and size (e.g., `HEAP_START = 0x_4444_4444_0000`, `HEAP_SIZE = 100 KiB`).
- This region is just virtual addresses; it's not backed by any real memory yet.

#### **5. Step 2: Mapping the Heap**

- **Problem:** If we try to use the heap region, the CPU will trigger a **page fault** because those virtual addresses aren't mapped to any physical frames.
- **Solution:** We must use our paging implementation (from the previous article) to map this virtual region.
- **Action:** We create an `init_heap` function. This function:
    1. Takes our `Mapper` and `FrameAllocator`.
    2. Calculates the range of virtual pages needed for the heap region.
    3. Loops over these pages.
    4. For each page, it:
        - Asks the `FrameAllocator` for a free **physical frame**.
        - Calls `mapper.map_to(...)` to map the virtual page to that physical frame.
        - Sets the page flags to `PRESENT | WRITABLE`.
- After `init_heap` runs, the virtual memory region `0x_4444_4444_0000` is now backed by real, writable physical memory.

#### **6. Step 3: Using an Allocator Crate**

- Writing a _good_ allocator is extremely complex. Instead of writing our own, we use a pre-built one.
- **Action 1 (Add Crate):** We add the `linked_list_allocator` crate. This crate provides a `LockedHeap` type that uses a simple linked list to keep track of free memory blocks. It's "locked" because it contains a `Mutex` (spinlock) to make it safe for concurrent use (though we must be careful not to use it in interrupt handlers to avoid deadlocks).
- **Action 2 (Register Allocator):** We replace our dummy allocator with the real one:
    ```rust
    use linked_list_allocator::LockedHeap;
    
    #[global_allocator]
    static ALLOCATOR: LockedHeap = LockedHeap::empty();
    ```
    
- **Action 3 (Initialize Allocator):** The allocator starts "empty." We must tell it where its memory is. After `init_heap` runs, we call:
    ```rust
    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }
    ```
- This one call gives the `linked_list_allocator` the 100 KiB memory region we just mapped, which it can now manage.

---

#### **7. The Result**

Once these steps are complete, we can finally use `Box`, `Vec`, and all other types from the `alloc` crate in our kernel. The kernel now has dynamic memory.