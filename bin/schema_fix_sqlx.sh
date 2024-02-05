
set -e

rm -rf ~/.rg/sqlx

cargo clean

cargo build -p redgold-data

cargo test -p redgold-data
