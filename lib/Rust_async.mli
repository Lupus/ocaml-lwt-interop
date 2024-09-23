module Runtime : sig
  type t

  val create : unit -> t
  val test : t -> unit Lwt.t
  val destroy : t -> unit Lwt.t

  module Private : sig
    type rust_executor

    val rust_executor_of_t : t -> rust_executor
  end
end
