##  1. The `Task` Struct: Our Unit of Work

We first define a `Task` struct. This is the "wrapper" for any future our executor needs to manage.
```rust
// in src/task/mod.rs
use core::{future::Future, pin::Pin};
use alloc::boxed::Box;

pub struct Task {
    future: Pin<Box<dyn Future<Output = ()>>>,
}
```

Let's break down the `future` field's type, `Pin<Box<dyn Future<Output = ()>>>`:

- `dyn Future<Output = ()>`: This is a "trait object." It means "any type that implements the `Future` trait and returns `()`. We use this because every `async fn` has its own unique, anonymous type. A trait object allows us to store different _kinds_ of futures (e.g., `async_keyboard_input` and `async_timer_task`) in the same `Task` struct.
- `Box<...>`: This allocates the `Future` on the **heap**. This is essential for two reasons:
    1. A `dyn Future`'s size isn't known at compile time, so it _must_ be stored behind a pointer (like `Box`).
    2. `async` state machines can get very large. Storing them on the stack of the `run` loop could cause a stack overflow. The heap has much more space.
- `Pin<...>`: This is the most critical part. We use `Box::pin` to create a **pinned** `Box`. This means the `Box`'s contents (our `Future`) are _guaranteed_ to never be moved to a different memory address. This is a safety requirement because a paused `Future` might contain pointers _to itself_, and moving it would invalidate those pointers and lead to undefined behavior.

---

## ## 2. The `Executor` Struct: The Task Manager

The `Executor` is the heart of the system. It's the struct that holds all the tasks and decides which one to run.
```rust
// in src/task/executor.rs
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use crossbeam_queue::ArrayQueue;

pub struct Executor {
    tasks: BTreeMap<TaskID, Task>,
    task_queue: Arc<ArrayQueue<TaskID>>,
    waker_cache: BTreeMap<TaskID, Waker>,
}
```

Here's what each field does:

- **`tasks: BTreeMap<TaskID, Task>`:** This is the "master list" of all tasks in the system. It's a `BTreeMap` (a sorted map) that lets us look up a `Task` struct using its unique `TaskID`. When a task is finished, it's removed from this map.
- **`task_queue: Arc<ArrayQueue<TaskID>>`:** This is the "ready" queue. It _only_ stores the `TaskID`s of tasks that are ready to be polled.
    - **`ArrayQueue`:** We use this type from the `crossbeam-queue` crate because it's a "lock-free" queue. This is **critically important**. It means that our interrupt handlers (like the keyboard) can _push_ a `TaskID` onto this queue without needing a `Mutex`. This avoids the deadlocks we discussed in the "Hardware Interrupts" post.
    - **`Arc` (Atomic Reference Counter):** This allows the `task_queue` to be "shared" between the `Executor` (which _pops_ from it) and the many `Waker`s (which _push_ to it). `Arc` ensures the queue itself lives as long as anyone is using it.
- **`waker_cache: BTreeMap<TaskID, Waker>`:** This is a performance optimization. Creating a `Waker` involves creating an `Arc`, which is a small allocation. To avoid re-creating a `Waker` every time we poll a task, we cache it here.

---

### ## 3. The `Waker`: The Signal for Work

The `Waker` is the magic that connects the interrupt handler back to the `Executor`
We create our own `Waker` by implementing the `ArcWake` trait for a custom struct.
```rust
// in src/task/waker.rs
struct TaskWaker {
    task_id: TaskID,
    task_queue: Arc<ArrayQueue<TaskID>>,
}

impl ArcWake for TaskWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // This is the waker's *only* job:
        // Push its TaskID into the shared "ready" queue.
        arc_self.task_queue.push(arc_self.task_id).expect("task_queue full");
    }
}
```

- When the keyboard interrupt fires, it knows the `TaskID` of the `keyboard_task` that is waiting.
- It gets that task's `Waker` and calls `.wake()`.
- This triggers `wake_by_ref`, which simply pushes the `TaskID` into the `task_queue`.
- The `Executor`'s loop, which was previously sleeping, now sees that the queue is no longer empty and knows it has work to do.

---

### ## 4. The `run()` Loop: The Engine

This is the main loop of the `Executor`. It ties everything together.
```rust
// in src/task/executor.rs
impl Executor {
    pub fn run(&mut self) -> ! {
        loop {
            // 1. Get a "ready" task from the queue.
            while let Some(task_id) = self.task_queue.pop() {
                // 2. Look up the Task struct from the master list.
                let task = match self.tasks.get_mut(&task_id) {
                    Some(task) => task,
                    None => continue, // task was already completed
                };

                // 3. Create a Waker and Context for this task.
                let waker = self.waker_cache
                    .entry(task_id)
                    .or_insert_with(|| TaskWaker::new(task_id, self.task_queue.clone()));
                let mut context = Context::from_waker(waker);

                // 4. POLL THE TASK
                match task.poll(&mut context) {
                    Poll::Pending => {
                        // Do nothing. The task is now idle.
                        // It will *only* run again if its Waker is called.
                    }
                    Poll::Ready(()) => {
                        // 5. Task is done! Remove it from the system.
                        self.tasks.remove(&task_id);
                        self.waker_cache.remove(&task_id);
                    }
                }
            }

            // 6. No more tasks are "ready". Put the CPU to sleep.
            self.sleep_if_idle();
        }
    }

    fn sleep_if_idle(&self) {
        // We use `without_interrupts` to prevent a race condition:
        // 1. Disable interrupts.
        // 2. Check if the task_queue is *still* empty.
        //    (An interrupt might have fired *just* before we disabled them)
        // 3. If it is empty, call `hlt`.
        // 4. If it's not empty, re-enable interrupts and loop again.
        interrupts::without_interrupts(|| {
            if self.task_queue.is_empty() {
                // The `hlt` instruction is safe *inside* this
                // `without_interrupts` block because it
                // atomically re-enables interrupts and sleeps.
                interrupts::enable_and_hlt();
            } else {
                // Interrupts are re-enabled by `without_interrupts`
            }
        });
    }
}
```