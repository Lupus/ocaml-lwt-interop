type mlbox_future
external lwti_mlbox_future_create: unit -> mlbox_future = "lwti_mlbox_future_create"
external lwti_mlbox_future_resolve: mlbox_future -> 'a -> unit = "lwti_mlbox_future_resolve"
external lwti_mlbox_future_reject: mlbox_future -> string -> unit = "lwti_mlbox_future_reject"

type executor
external lwti_executor_create: int -> executor = "lwti_executor_create"
external lwti_executor_run_pending: executor -> unit = "lwti_executor_run_pending"
external lwti_executor_bench: unit -> unit Lwt.t = "lwti_executor_bench"
external lwti_executor_test: unit -> unit Lwt.t = "lwti_executor_test"
