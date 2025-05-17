# AGENTS.md

## Project Overview
This repository contains a mixed OCaml/Rust project where Rust bindings are exposed to OCaml. Understanding this hybrid nature is essential when working with the codebase.

## Environment Constraints
- You are operating in an offline environment without internet access
- ALWAYS add `--offline` flag to all cargo commands
- Do not attempt to modify or update dependencies as they cannot be downloaded
- Do not try to install new packages or dependencies

## Code Style & Formatting
- Always format Rust code before submitting changes: `cargo fmt --all`
- All Clippy warnings must be fixed (they are treated as errors in CI)
- Use automatic fixing when possible: `cargo clippy --fix --all -- -D warnings --offline`
- Always format Rust code before submitting changes: `opam exec -- dune fmt`

## Testing Requirements
- Run Rust tests with: `cargo test --offline`
- Run OCaml integration tests with: `opam exec -- dune runtest`
- Both test suites must pass for any changes
- Rust tests focus on internal functionality within the Rust domain
- OCaml tests verify end-to-end integration between Rust and OCaml

## Build Process
1. Format code: `cargo fmt --all`/`opam exec -- dune fmt`
2. Fix linting issues: `cargo clippy --fix --all -- -D warnings --offline`
3. Run Rust tests: `cargo test --offline`
4. Build OCaml components: `opam exec -- dune build`
5. Run OCaml tests: `opam exec -- dune runtest`

## PR Instructions
- Title format: [Component] Brief description
- Include a "Testing Done" section that lists verification steps performed
- Ensure all tests pass locally before submission
