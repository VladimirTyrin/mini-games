#/usr/bin/env bash

CARGO_PROFILE_BENCH_DEBUG=true cargo build --release && CARGO_PROFILE_BENCH_DEBUG=true cargo flamegraph --bench minimax_bench
