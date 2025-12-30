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

alias r := run
alias b := build
alias t := test
alias l := lint
alias f := format
alias cov := coverage
