set -e
export RUSTFLAGS="-C link-arg=-fuse-ld=lld -A warnings";
cargo build
./target/debug/redgold --offline gui
