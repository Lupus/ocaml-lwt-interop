(function() {
    var implementors = Object.fromEntries([["ocaml_lwt_interop",[["impl&lt;Args, Ret&gt; OCamlDesc for <a class=\"struct\" href=\"ocaml_lwt_interop/async_func/struct.OCamlAsyncFunc.html\" title=\"struct ocaml_lwt_interop::async_func::OCamlAsyncFunc\">OCamlAsyncFunc</a>&lt;Args, Ret&gt;<div class=\"where\">where\n    Args: Callable&lt;<a class=\"struct\" href=\"ocaml_lwt_interop/promise/struct.Promise.html\" title=\"struct ocaml_lwt_interop::promise::Promise\">Promise</a>&lt;Ret&gt;&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a>,\n    Ret: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a>,\n    <a class=\"struct\" href=\"ocaml_lwt_interop/promise/struct.Promise.html\" title=\"struct ocaml_lwt_interop::promise::Promise\">Promise</a>&lt;Ret&gt;: FromValue + OCamlDesc + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a>,</div>"],["impl&lt;T&gt; OCamlDesc for <a class=\"struct\" href=\"ocaml_lwt_interop/promise/struct.Promise.html\" title=\"struct ocaml_lwt_interop::promise::Promise\">Promise</a>&lt;T&gt;<div class=\"where\">where\n    T: OCamlDesc,</div>"]]]]);
    if (window.register_implementors) {
        window.register_implementors(implementors);
    } else {
        window.pending_implementors = implementors;
    }
})()
//{"start":57,"fragment_lengths":[1284]}