module Promise : sig
  type 'a t

  val of_lwt : 'a Lwt.t -> 'a t
end

module Runtime : sig
  type t

  val create : unit -> t
  val test : t -> f:(int -> unit Promise.t) -> unit
  val destroy : t -> unit Lwt.t

  module Private : sig
    type rust_executor

    val rust_executor_of_t : t -> rust_executor
  end
end
