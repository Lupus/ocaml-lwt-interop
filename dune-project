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
  dune
  conf-rust-2021
  (rust-staticlib-gen
   (>= 0.2.2))
  (rust-staticlib-virtual
   (>= 0.2.2))
  (dune-cargo-build
   (and
    (>= 0.2.2)
    :build))
  (ocaml-rs-smartptr
   (>= 0.1.0))
  (lwt
   (and
    (>= 5.6.0)
    (< 6.0.0)))
  (lwt_ppx
   (and
    (>= 5.8.0)
    (< 6.0.0)))
  (alcotest
   (>= 1.9.0))
  (alcotest-lwt
   (>= 1.9.0))
  (ocamlformat
   (and
    :with-test
    (>= 0.26.2)
    (< 0.27.0)))
  (odoc :with-doc)))
