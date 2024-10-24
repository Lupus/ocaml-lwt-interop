
module Tests = struct 
  external bench : unit -> ((unit) Lwt.t) = "lwti_tests_bench"
  external test_1 : unit -> ((unit) Lwt.t) = "lwti_tests_test1"
  external test_2 : ((unit) -> (((unit) Lwt.t))) -> ((unit) Lwt.t) = "lwti_tests_test2"
  external test_sync_call : ((unit) -> (unit)) -> ((unit) Lwt.t) = "lwti_tests_test_sync_call"
end

