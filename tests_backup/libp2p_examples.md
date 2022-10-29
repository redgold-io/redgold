
/*
cargo run --bin file-sharing -- \
--listen-address /ip4/127.0.0.1/tcp/40837 \
--secret-key-seed 1 \
provide \
--path trust_notes.md \
--name a

cargo run --bin file-sharing -- \
--peer /ip4/127.0.0.1/tcp/40837/p2p/12D3KooWPjceQrSwdWXPyLLeABRXmuqt69Rg3sBYbU1Nft9HyQ6X \
get \
--name a

*/