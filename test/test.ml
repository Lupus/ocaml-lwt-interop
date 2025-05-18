open Alcotest
open Alcotest_lwt
open Lwt.Infix
open Stubs

let test_bench _ () = Tests.bench () >|= fun () -> ()
let test_test1 _ () = Tests.test_1 () >|= fun () -> ()
let test_test2 _ () = Tests.test_2 (fun () -> Lwt.return_unit) >|= fun () -> ()

let test_sync_call _ () =
  let called = ref false in
  Tests.test_sync_call (fun () -> called := true)
  >>= fun () ->
  check bool "callback called" true !called;
  Lwt.return_unit
;;

let test_spawn_lwt _ () =
  Tests.spawn_lwt 41L
  >>= fun v ->
  check int64 "value" 42L v;
  Lwt.return_unit
;;

let test_run_in_ocaml_domain _ () =
  let called = ref false in
  Tests.run_in_ocaml_domain (fun () -> called := true)
  >>= fun () ->
  check bool "callback called" true !called;
  Lwt.return_unit
;;

let test_handle _ () =
  let called = ref false in
  Tests.handle_test (fun () ->
    called := true;
    Lwt.return_unit)
  >>= fun () ->
  check bool "async func called" true !called;
  Lwt.return_unit
;;

let test_promise_from_rust _ () =
  Tests.promise_create 5L
  >>= fun v ->
  check int64 "promise" 5L v;
  Lwt.catch
    (fun () -> Tests.promise_create_err "boom" >>= fun _ -> fail "expected exn")
    (fun _ -> Lwt.return_unit)
;;

let test_promise_to_rust _ () =
  let p, w = Lwt.wait () in
  Lwt.wakeup_later w 7L;
  Tests.await_promise p
  >>= function
  | Ok v ->
    check int64 "await" 7L v;
    Lwt.return_unit
  | Error msg -> fail msg
;;

let test_promise_to_rust_err _ () =
  let p, w = Lwt.wait () in
  Lwt.wakeup_later_exn w (Failure "err");
  Tests.await_promise p
  >>= function
  | Ok _ -> fail "expected error"
  | Error _ -> Lwt.return_unit
;;

let () =
  Lwt_main.run
    (run
       "ocaml-lwt-interop"
       [ ( "basic"
         , [ test_case "bench" `Quick test_bench
           ; test_case "test1" `Quick test_test1
           ; test_case "test2" `Quick test_test2
           ; test_case "sync_call" `Quick test_sync_call
           ; test_case "spawn_lwt" `Quick test_spawn_lwt
           ; test_case "run_in_ocaml_domain" `Quick test_run_in_ocaml_domain
           ; test_case "handle" `Quick test_handle
           ; test_case "promise_from_rust" `Quick test_promise_from_rust
           ; test_case "promise_to_rust" `Quick test_promise_to_rust
           ; test_case "promise_to_rust_err" `Quick test_promise_to_rust_err
           ] )
       ])
;;
