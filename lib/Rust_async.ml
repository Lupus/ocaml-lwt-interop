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
    Stubs.lwti_executor_run_pending executor;
    { executor; notification }
  ;;

  let bench t = Stubs.lwti_executor_bench t.executor
  let test t = Stubs.lwti_executor_test t.executor

  let destroy t =
    Lwt_unix.stop_notification t.notification;
    Lwt.return ()
  ;;

  module Private = struct
    type rust_executor = Stubs.executor

    let rust_executor_of_t { executor; _ } = executor
  end
end

let () =
  Callback.register "olwti_lwt_task" Lwt.task;
  Callback.register "olwti_lwt_wakeup_later" Lwt.wakeup_later;
  Callback.register "olwti_lwt_wakeup_later_exn" (fun resolver msg ->
    Lwt.wakeup_later_exn resolver (Failure msg));
  Callback.register "olwti_printexc_to_string" Printexc.to_string
;;
