
set -e
cargo build
rm -rf ~/.rg/dev
./target/debug/redgold --network dev --genesis node --live-e2e-interval 5
