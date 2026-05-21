# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build          # build
cargo run            # run
cargo test           # run all tests
cargo test <name>    # run single test by name
cargo clippy         # lint
cargo fmt            # format
```

## Project

Rust CLI tool (`parket`, edition 2024) — early stage, `src/main.rs` is the only source file. Sample `.parquet` files (`sample1-3.parquet`) are present in the repo root, suggesting the tool will work with Parquet data.
