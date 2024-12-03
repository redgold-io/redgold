export RUST_MIN_STACK=327772160;

# Check if system is macOS
if [[ "$(uname)" == "Darwin" ]]; then
    export RUSTFLAGS="-A warnings"
else
    export RUSTFLAGS="-C link-arg=-fuse-ld=lld -A warnings"
fi

cargo test --lib --profile ci -- --nocapture