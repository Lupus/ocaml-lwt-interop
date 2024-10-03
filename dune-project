(lang dune 3.7)

(name rust-async)

(generate_opam_files true)

(package
 (name rust-async)
 (synopsis "Interop between OCaml Lwt and Rust async")
 (license "Apache License 2.0")
 (maintainers "Konstantin Olkhovskiy <lupus@oxnull.net>")
 (authors "The OCaml Lwt Interop contributors")
 (homepage "https://github.com/Lupus/ocaml-lwt-interop")
 (bug_reports "https://github.com/Lupus/ocaml-lwt-interop/issues")
 (depends
  (ocaml
   (< 5.0.0))
  dune
  conf-rust-2021
  (rust-staticlib
   (>= ~dev))
  (ocaml-rs-smartptr
   (>= 0.1.3))
  (lwt
   (and
    (>= 5.6.0)
    (< 5.7.0)))
  (lwt_ppx
   (and
    (>= 2.1.0)
    (< 2.2.0)))
  (ocamlformat
   (and
    :with-test
    (>= 0.25.1)
    (< 0.26.0)))
  (odoc :with-doc)))
