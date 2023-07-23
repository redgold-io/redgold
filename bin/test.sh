# TODO: Remove --test-threads when extraneous test parallelism is fixed
export RUST_MIN_STACK=20485760; cargo test --lib -- --test-threads=1