# TODO: Remove --test-threads when extraneous test parallelism is fixed
export RUST_MIN_STACK=20485760; export RUSTFLAGS="-C link-arg=-fuse-ld=lld"; cargo test --lib -- --nocapture