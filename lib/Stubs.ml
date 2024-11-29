module Future = struct
  type tags =
    [ `Ocaml_lwt_interop_ml_box_future_ml_box_future
    | `Core_marker_sync
    | `Core_marker_send
    ]

  type 'a t' = ([> tags ] as 'a) Ocaml_rs_smartptr.Rusty_obj.t
  type t = tags t'

  external create : unit -> _ t' = "lwti_mlbox_future_create"
  external resolve : _ t' -> 'a -> unit = "lwti_mlbox_future_resolve"
  external reject : _ t' -> string -> unit = "lwti_mlbox_future_reject"
end

module Executor = struct
  type tags =
    [ `Ocaml_lwt_interop_domain_executor_domain_executor
    | `Core_marker_sync
    | `Core_marker_send
    ]

  type 'a t' = ([> tags ] as 'a) Ocaml_rs_smartptr.Rusty_obj.t
  type t = tags t'

  external create : int -> _ t' = "lwti_executor_create"
  external run_pending : _ t' -> unit = "lwti_executor_run_pending"
end
