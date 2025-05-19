(function() {
    var implementors = Object.fromEntries([["ocaml_lwt_interop",[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a> for <a class=\"struct\" href=\"ocaml_lwt_interop/ml_box_future/struct.MlBoxFuture.html\" title=\"struct ocaml_lwt_interop::ml_box_future::MlBoxFuture\">MlBoxFuture</a>"],["impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a> for <a class=\"struct\" href=\"ocaml_lwt_interop/promise/struct.PromiseFuture.html\" title=\"struct ocaml_lwt_interop::promise::PromiseFuture\">PromiseFuture</a>&lt;T&gt;<div class=\"where\">where\n    T: FromValue + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.86.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'static,</div>"]]]]);
    if (window.register_implementors) {
        window.register_implementors(implementors);
    } else {
        window.pending_implementors = implementors;
    }
})()
//{"start":57,"fragment_lengths":[893]}