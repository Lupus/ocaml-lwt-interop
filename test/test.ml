open Stubs

let main_rust () =
  print_endline "";
  print_endline "running Lwt+Rust test";
  let start = Unix.gettimeofday () in
  let pause = Lwt_unix.auto_pause 0.1 in
  let page = ref 0 in
  let rec aux x =
    let%lwt () = Tests.bench () in
    page := x;
    let%lwt () = pause () in
    aux (x + 1)
  in
  let test () = Lwt.async (fun () -> aux 0) in
  test ();
  print_endline "lwt sleeping";
  let%lwt () = Lwt_unix.sleep 10.0 in
  let finish = Unix.gettimeofday () in
  Printf.printf
    "%.3f iterations per second, %d iterations total [Rust+Lwt]\n"
    (float_of_int !page /. (finish -. start))
    !page;
  print_endline "lwt main returning";
  Lwt.return ()
;;

let main_rust_slow () =
  print_endline "";
  print_endline "running Lwt+Rust (slow) test";
  let start = Unix.gettimeofday () in
  let pause = Lwt_unix.auto_pause 0.1 in
  let page = ref 0 in
  let rec aux x =
    let%lwt () =
      Tests.test_2 (fun () ->
        page := x;
        pause ())
    in
    aux (x + 1)
  in
  let test () = Lwt.async (fun () -> aux 0) in
  test ();
  print_endline "lwt sleeping";
  let%lwt () = Lwt_unix.sleep 10.0 in
  let finish = Unix.gettimeofday () in
  Printf.printf
    "%.3f iterations per second, %d iterations total [Rust+Lwt]\n"
    (float_of_int !page /. (finish -. start))
    !page;
  print_endline "lwt main returning";
  Lwt.return ()
;;

let main_gc () =
  print_endline "";
  print_endline "running GC smoke test";
  let start = Unix.gettimeofday () in
  let page = ref 0 in
  let rec aux x =
    let%lwt () =
      Tests.test_2 (fun () ->
        Gc.full_major ();
        page := x;
        Gc.full_major ();
        let%lwt () = Lwt.pause () in
        Gc.full_major ();
        Lwt.return ())
    in
    Gc.full_major ();
    aux (x + 1)
  in
  let test () = Lwt.async (fun () -> aux 0) in
  test ();
  print_endline "lwt sleeping";
  Gc.full_major ();
  let%lwt () = Lwt_unix.sleep 10.0 in
  let finish = Unix.gettimeofday () in
  Printf.printf
    "%.3f iterations per second, %d iterations total [GC smoke]\n"
    (float_of_int !page /. (finish -. start))
    !page;
  print_endline "exiting test function (pending GC in 0.5 seconds)";
  let fut = Lwt_unix.sleep 0.5 in
  Lwt.on_success fut (fun () ->
    print_endline "final GC run";
    Gc.full_major ());
  fut
;;

let main_lwt () =
  print_endline "";
  print_endline "running Lwt-only baseline";
  let start = Unix.gettimeofday () in
  let page = ref 0 in
  let pause = Lwt_unix.auto_pause 0.1 in
  let rec aux f x =
    let%lwt () = Sys.opaque_identity (f x) in
    aux f (x + 1)
  in
  let test ~f = Lwt.async (fun () -> aux f 0) in
  test ~f:(fun p ->
    page := p;
    pause ());
  print_endline "lwt sleeping";
  let%lwt () = Lwt_unix.sleep 10.0 in
  let finish = Unix.gettimeofday () in
  Printf.printf
    "%.3f iterations per second, %d iterations total [Only Lwt]\n"
    (float_of_int !page /. (finish -. start))
    !page;
  print_endline "lwt main returning";
  Lwt.return ()
;;

let main_sync () =
  print_endline "";
  print_endline "running Sync func call test";
  let start = Unix.gettimeofday () in
  let page = ref 0 in
  let rec aux f x =
    let%lwt () = f x in
    if x = 50_000 then Lwt.return () else aux f (x + 1)
  in
  let test p =
    let%lwt () = Tests.test_sync_call (fun () -> page := p) in
    Lwt.return ()
  in
  print_endline "running the test";
  let%lwt () = aux test 0 in
  let finish = Unix.gettimeofday () in
  Printf.printf
    "%.3f iterations per second, %d iterations total [Sync func call]\n"
    (float_of_int !page /. (finish -. start))
    !page;
  print_endline "test main returning";
  Lwt.return ()
;;

let () =
  Lwt_main.run
    (match Sys.argv with
     | [| _; "lwt" |] -> main_lwt ()
     | [| _; "rust" |] -> main_rust ()
     | [| _; "rust-slow" |] -> main_rust_slow ()
     | [| _; "gc" |] -> main_gc ()
     | [| _; "sync" |] -> main_sync ()
     | [| _ |] ->
       failwith "no command provided on command line - should be one of: lwt, rust, gc"
     | _ -> failwith "unknown command line arguments")
;;
