# TODO: Remove --test-threads when extraneous test parallelism is fixed
export RUST_MIN_STACK=10485760; cargo test --lib -- --test-threads=1