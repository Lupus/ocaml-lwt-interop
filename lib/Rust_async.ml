module Promise = struct
  type 'a t = Stubs.promise

  let create () = Stubs.lwti_promise_create ()

  let of_lwt fut =
    let p = create () in
    Lwt.on_any
      fut
      (fun value -> Stubs.lwti_promise_resolve p value)
      (fun exn -> Stubs.lwti_promise_reject p exn);
    p
  ;;
end

module Runtime = struct
  type t =
    { executor : Stubs.executor
    ; notification : int
    }

  let create () =
    let notification = Lwt_unix.make_notification ~once:false Fun.id in
    let executor = Stubs.lwti_executor_create notification in
    Lwt_unix.set_notification notification (fun () ->
      Stubs.lwti_executor_run_pending executor);
    { executor; notification }
  ;;

  let test t ~f = Stubs.lwti_executor_test t.executor f

  let destroy t =
    Lwt_unix.stop_notification t.notification;
    Lwt.return ()
  ;;

  module Private = struct
    type rust_executor = Stubs.executor

    let rust_executor_of_t { executor; _ } = executor
  end
end
