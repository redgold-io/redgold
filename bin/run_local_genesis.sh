
set -e
cargo build
rm -rf ~/.rg/local
./target/debug/redgold --network local --genesis node --live-e2e-interval 5
