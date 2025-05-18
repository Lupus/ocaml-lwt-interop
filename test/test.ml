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
           ] )
       ])
;;
