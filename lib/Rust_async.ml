module Runtime = struct
  type t =
    { executor : Stubs.Executor.t
    ; notification : int
    }

  (* For OCaml 5 this should be domain-local storage, for OCaml 4 global ref is
     fine, as in OCaml 4 there's only one domain *)
  let current = ref None

  let create () =
    let notification = Lwt_unix.make_notification ~once:false Fun.id in
    let executor = Stubs.Executor.create notification in
    Lwt_unix.set_notification notification (fun () -> Stubs.Executor.run_pending executor);
    Stubs.Executor.run_pending executor;
    let t = { executor; notification } in
    Gc.finalise
      (fun { executor = _; notification } -> Lwt_unix.stop_notification notification)
      t;
    t
  ;;

  let current () =
    if not (Domain_compat.is_main_domain ())
    then
      failwith
        "Initializing Rust_async executor from non-main domain is not going to work well";
    match !current with
    | Some executor -> executor
    | None ->
      let executor = create () in
      current := Some executor;
      executor
  ;;
end

let () =
  (* Below callbacks are used in ../src/promise.rs and ../src/domain_executor.rs *)
  Callback.register "olwti_lwt_task" Lwt.task;
  Callback.register "olwti_lwt_wakeup_later" (fun resolver v ->
    try Ok (Lwt.wakeup_later resolver v) with
    | e -> Error ("Lwt.wakup_later failed: " ^ Printexc.to_string e));
  Callback.register "olwti_lwt_wakeup_later_exn" (fun resolver msg ->
    try Ok (Lwt.wakeup_later_exn resolver (Failure msg)) with
    | e -> Error ("Lwt.wakup_later_exn failed: " ^ Printexc.to_string e));
  Callback.register "olwti_current_executor" (fun () ->
    let current = Runtime.current () in
    current.executor);
  Callback.register "olwti_wrap_lwt_future" (fun fut ->
    let wrapper = Stubs.Future.create () in
    Lwt.on_any
      fut
      (fun value -> Stubs.Future.resolve wrapper value)
      (fun exn -> Stubs.Future.reject wrapper (Printexc.to_string exn));
    wrapper)
;;
