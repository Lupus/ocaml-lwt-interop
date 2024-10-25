# OCaml Lwt Interop

**WARNING**: Highly experimental code, do not use in production!

This project aims to solve the problem of interop between asynchronous OCaml (Lwt flavor) and asynchronous Rust.

## Architecture

### Domain Executor

The `DomainExecutor` runs within an OCaml domain and integrates Rust async tasks with the OCaml runtime. It uses the `async_executor` crate to manage async tasks and integrates with the `tokio` runtime for asynchronous I/O operations. The executor ensures that the OCaml runtime lock is properly managed during task execution.

### Rust `Promise` and `Future` Integration

To bridge OCaml's Lwt promises with Rust's async/await syntax, the project provides a `Promise` type that implements Rust's `Future` trait. This allows Rust code to await OCaml promises asynchronously. The OCaml side can create a promise, return it to Rust, and later resolve or reject it with a value. The value is stored in the `Promise`, and when a Rust task polls this `Promise`, it will get the value back or be woken up if the value is not yet available.

## Test Scenario

The test scenario can be found in `test/test.ml`. It passes an Lwt-enabled callback into a Rust async task, which executes the callback, waits for it to complete, and then loops over calling it again.

```mermaid
sequenceDiagram
participant lwt_loop as Lwt Event Loop (OCaml)
participant fclo as F Closure (OCaml)
participant lwt_main as Lwt Main (OCaml)
participant rust_test as Runtime test func (Rust)
participant runtime as Runtime (Rust)
participant task as Task (Rust)

activate lwt_main
note right of lwt_main: creates Rust runtime
note right of lwt_main: creates OCaml Closure F (Lwt-enabled)
lwt_main ->> rust_test: Run test function and pass F
activate rust_test
rust_test ->> runtime: Spawns async task calling F in a loop
activate runtime
runtime -->> lwt_loop: Trigger Lwt_unix notification
note right of runtime: Task is stored in runtime state
runtime ->> rust_test: Returns
deactivate runtime
rust_test ->> lwt_main: Returns
deactivate rust_test
note right of lwt_main: Sleeps...
deactivate lwt_main

note right of lwt_loop: Process Lwt_unix notification
lwt_loop ->> runtime: Run pending tasks
activate lwt_loop
activate runtime
runtime ->> task: Run

loop Forever

activate task
task ->> fclo: Calls F Closure
activate fclo
note right of fclo: Creates new Lwt.pause promise<br/>and links it with Rust promise
fclo -->> lwt_loop: Lwt.pause
fclo ->> task: Rust promise (implements Future)
deactivate fclo
note left of task: Promise .await
task ->> runtime: Return
deactivate task
runtime ->> lwt_loop: Return
deactivate runtime
deactivate lwt_loop

note right of lwt_loop: Process Lwt.pause at next tick
lwt_loop -->> task: Wakeup promise
activate lwt_loop
task -->> runtime: Add task to pending
runtime -->> lwt_loop: Trigger Lwt_unix notification
deactivate lwt_loop

note right of lwt_loop: Process Lwt_unix notification
lwt_loop ->> runtime: Run pending tasks
activate lwt_loop
activate runtime
runtime ->> task: Run
activate task
note left of task: Promise .await completed
deactivate lwt_loop

end
```
