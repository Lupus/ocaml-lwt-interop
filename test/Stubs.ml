module Tests = struct
  external bench : unit -> unit Lwt.t = "lwti_tests_bench"
  external test_1 : unit -> unit Lwt.t = "lwti_tests_test1"
  external test_2 : (unit -> unit Lwt.t) -> unit Lwt.t = "lwti_tests_test2"
  external test_sync_call : (unit -> unit) -> unit Lwt.t = "lwti_tests_test_sync_call"
  external spawn_lwt : int64 -> int64 Lwt.t = "lwti_tests_spawn_lwt"
  external spawn_lwt_err : int64 -> int64 Lwt.t = "lwti_tests_spawn_lwt_err"

  external run_in_ocaml_domain
    :  (unit -> unit)
    -> unit Lwt.t
    = "lwti_tests_run_in_ocaml_domain"

  external handle_test : (unit -> unit Lwt.t) -> unit Lwt.t = "lwti_tests_handle"
  external promise_create : int64 -> int64 Lwt.t = "lwti_tests_promise_create"
  external promise_create_err : string -> int64 Lwt.t = "lwti_tests_promise_create_err"

  external await_promise
    :  int64 Lwt.t
    -> (int64, string) result Lwt.t
    = "lwti_tests_await_promise"
end
