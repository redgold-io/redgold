cargo build

pkill -f redgold
rm -rf ~/.rg/local
sleep 1
export REDGOLD_BINARY_PATH="./target/debug/redgold"
export RUST_BACKTRACE=1

$REDGOLD_BINARY_PATH --network local --debug-id 0 --genesis &
export NODE_1_PID=$!

sleep 7

$REDGOLD_BINARY_PATH --network local --debug-id 1 --seed-address 127.0.0.1 &
export NODE_2_PID=$!

sleep 7

$REDGOLD_BINARY_PATH --network local --debug-id 2 --seed-address 127.0.0.1 &
export NODE_3_PID=$!

sleep 7

cargo test local_e2e_it -- --nocapture
export TEST_EXIT_CODE=$?

kill -KILL $NODE_1_PID
kill -KILL $NODE_2_PID
kill -KILL $NODE_3_PID

exit $TEST_EXIT_CODE
