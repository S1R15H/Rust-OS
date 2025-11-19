## 🌀 Multitasking: Two Approaches

There are two main ways to run multiple tasks "at once":

- **Preemptive Multitasking (Threads):** The operating system is in control. It uses a hardware timer to forcibly pause (preempt) a task at any time and switch to another. This is powerful but has higher overhead, as each task needs its own separate stack.
- **Cooperative Multitasking (Async/Await):** The tasks are in control. A task runs until it _voluntarily_ gives up control, or "yields" (e.g., when it's waiting for something). This is much more lightweight and memory-efficient.

This article implements **cooperative multitasking**.

## 💡 The `Future` Trait
In Rust, `async`/`await` is built on a single, core concept: the `Future` trait.

- A **`Future`** is an object representing a value that might not be ready yet.
- It has one main function: **`poll`**.
- The `poll` function is called to "check" on the future. It returns one of two things:
    - `Poll::Ready(value)`: The value is ready! The operation is complete.
    - `Poll::Pending`: The value is _not_ ready yet. The task should yield (give up control) and be polled again later.

## ⚙️ `async` and `await`: The State Machine

When you write an `async fn`, the compiler transforms your code into a **state machine** (which is just an object that implements the `Future` trait).

- `async fn` does _not_ run the code; it just creates the state machine object.
- `await` is a "yield point." It's where the state machine pauses.
- The `Future` (the state machine) does **nothing** until its `poll` method is called.

## 📌 `Pin`: Preventing Movement

There's one major problem: when a task is paused at an `.await`, it might be storing pointers _to itself_ (e.g., a local variable on its "virtual" stack).

- **The Problem:** If we _moved_ that `Future` object in memory to a new location, all those internal pointers would be invalid and would crash the program.
- **The Solution:** `Pin` is a wrapper type that "pins" an object to its location in memory, guaranteeing it will never be moved. This makes it safe to poll.

## 🏃 The Executor and the Waker

These two components work together to run the futures.

1. **The Executor:** This is the "task runner." Its job is to hold a list of all active `Future`s and to call `poll` on them.
2. **The Problem:** A simple executor would just loop forever, polling every task over and over. This is _very_ inefficient if a task is "pending" for a long time (like waiting for a key press).
3. **The Solution (Waker):** The `poll` function is given a `Context` which contains a `Waker`.
    
    - If a task is `Pending`, it should store the `Waker`.
    - When the event it's waiting for happens (e.g., a key is pressed), that event's code calls `.wake()` on the stored `Waker`.
    - The `Waker` then tells the `Executor`, "This task is ready to make progress."
    - The `Executor` schedules the task to be polled again.

This system is highly efficient. The `Executor` only polls tasks when they _actually have work to do_.