export RUST_MIN_STACK=10485760
# TODO: Remove --test-threads when extraneous test parallelism is fixed
cargo test --lib -- --test-threads=1