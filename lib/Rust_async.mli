module Runtime : sig
  val test : unit -> unit Lwt.t
  val bench : unit -> unit Lwt.t
end
