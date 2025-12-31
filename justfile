recipe-list:
  just --list --unsorted

run:
  cargo run

build:
  cargo build

lint:
  cargo clippy --all-targets --all-features -- -D warnings

format:
  cargo fmt

test:
  cargo test

coverage:
  cargo llvm-cov

bench:
  cargo bench -p jpp_bench

bench-filter FILTER:
  cargo bench -p jpp_bench -- {{FILTER}}

setup:
  cargo install cargo-llvm-cov --locked

wasm-build:
  cargo build -p jpp_wasm --target wasm32-unknown-unknown --release
  wasm-bindgen --target web --out-dir web/wasm \
    target/wasm32-unknown-unknown/release/jpp_wasm.wasm

web-dev: wasm-build
  cd web && bun install && bunx --bun vite

web-build: wasm-build
  cd web && bun install && bunx --bun vite build

alias r := run
alias b := build
alias t := test
alias l := lint
alias f := format
alias cov := coverage
