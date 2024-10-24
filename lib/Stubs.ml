
module Future = struct 
  type nonrec 'a t = [ `Ocaml_lwt_interop_ml_box_future_ml_box_future|`Core_marker_sync|`Core_marker_send ] Ocaml_rs_smartptr.Rusty_obj.t
  external create : unit -> ('a t) = "lwti_mlbox_future_create"
  external resolve : ('a t) -> 'a -> unit = "lwti_mlbox_future_resolve"
  external reject : ('a t) -> string -> unit = "lwti_mlbox_future_reject"
end


module Executor = struct 
  type nonrec t = [ `Ocaml_lwt_interop_domain_executor_domain_executor|`Core_marker_sync|`Core_marker_send ] Ocaml_rs_smartptr.Rusty_obj.t
  external create : int -> t = "lwti_executor_create"
  external run_pending : t -> unit = "lwti_executor_run_pending"
end

