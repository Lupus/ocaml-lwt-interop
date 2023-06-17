module Promise = struct
  type 'a t = Rust.promise

  let create () = Rust.lwti_promise_create ()

  let of_lwt fut =
    let p = create () in
    Lwt.on_any
      fut
      (fun value -> Rust.lwti_promise_resolve p value)
      (fun exn -> Rust.lwti_promise_reject p exn);
    p
  ;;
end

module Runtime = struct
  type t =
    { executor : Rust.executor
    ; notification : int
    }

  let create () =
    let notification = Lwt_unix.make_notification ~once:false Fun.id in
    let executor = Rust.lwti_executor_create notification in
    Gc.finalise (fun _ -> Printf.eprintf "finalizing executor\n%!") executor;
    Lwt_unix.set_notification notification (fun () ->
      Rust.lwti_executor_run_pending executor);
    { executor; notification }
  ;;

  let test t ~f = Rust.lwti_executor_test t.executor f

  let destroy t =
    Lwt_unix.stop_notification t.notification;
    Lwt.return ()
  ;;

  module Private = struct
    type rust_executor = Rust.executor

    let rust_executor_of_t { executor; _ } = executor
  end
end
