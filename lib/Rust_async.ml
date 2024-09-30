module Runtime = struct
  type t =
    { executor : Stubs.executor
    ; notification : int
    }

  (* For OCaml 5 this should be domain-local storage, for OCaml 4 global ref is
  fine, as in OCaml 4 there's only one domain  *)
  let current = ref None

  let create () =
    let notification = Lwt_unix.make_notification ~once:false Fun.id in
    let executor = Stubs.lwti_executor_create notification in
    Lwt_unix.set_notification notification (fun () ->
      Stubs.lwti_executor_run_pending executor);
    Stubs.lwti_executor_run_pending executor;
    let t = { executor; notification } in
    Gc.finalise
      (fun { executor = _; notification } -> Lwt_unix.stop_notification notification)
      t;
    t
  ;;

  let current () =
    match !current with
    | Some executor -> executor
    | None ->
      let executor = create () in
      current := Some executor;
      executor
  ;;

  let bench () = Stubs.lwti_executor_bench ()
  let test () = Stubs.lwti_executor_test ()
end

let () =
  Callback.register "olwti_lwt_task" Lwt.task;
  Callback.register "olwti_lwt_wakeup_later" Lwt.wakeup_later;
  Callback.register "olwti_lwt_wakeup_later_exn" (fun resolver msg ->
    Lwt.wakeup_later_exn resolver (Failure msg));
  Callback.register "olwti_current_executor" (fun () ->
    let current = Runtime.current () in
    current.executor);
  Callback.register "olwti_wrap_lwt_future" (fun fut ->
    let wrapper = Stubs.lwti_mlbox_future_create () in
    Lwt.on_any
      fut
      (fun value -> Stubs.lwti_mlbox_future_resolve wrapper value)
      (fun exn -> Stubs.lwti_mlbox_future_reject wrapper (Printexc.to_string exn));
    wrapper)
;;
