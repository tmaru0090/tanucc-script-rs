setlocal
set RUST_LOG=debug
cargo test all_default_run --features "full,wip-system"  -- --nocapture
endlocal
