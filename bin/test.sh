export RUST_MIN_STACK=327772160;
export RUSTFLAGS="-C link-arg=-fuse-ld=lld -A warnings";
cargo test --lib --profile ci -- --nocapture