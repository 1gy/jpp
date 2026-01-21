recipe-list:
  just --list --unsorted

run:
  cargo run

build:
  cargo build --workspace

lint:
  cargo clippy --all-targets --all-features -- -D warnings

format:
  cargo fmt --all

test:
  cargo test --workspace

coverage:
  cargo llvm-cov

bench:
  cargo bench -p jpp_bench

bench-filter FILTER:
  cargo bench -p jpp_bench -- {{FILTER}}

setup:
  cargo install cargo-llvm-cov --locked

wasm-build:
  wasm-pack build crates/jpp_wasm --target web --out-dir ../../web/wasm --release

web-dev: wasm-build
  cd web && bun install && bunx --bun vite --host

web-build: wasm-build
  cd web && bun install && bunx --bun vite build

alias r := run
alias b := build
alias t := test
alias l := lint
alias f := format
alias cov := coverage
