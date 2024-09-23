(* Generated by ocaml-rs *)

open! Bigarray

(* file: stubs.rs *)

type promise
type executor
external lwti_promise_create: unit -> promise = "lwti_promise_create"
external lwti_promise_resolve: promise -> 'a -> unit = "lwti_promise_resolve"
external lwti_promise_reject: promise -> exn -> unit = "lwti_promise_reject"
external lwti_executor_create: int -> executor = "lwti_executor_create"
external lwti_executor_run_pending: executor -> unit = "lwti_executor_run_pending"
external lwti_executor_test: executor -> (int -> promise) -> unit = "lwti_executor_test"