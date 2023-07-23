
set -e
cargo build
./target/debug/redgold --network dev --genesis node --live-e2e-interval 5
