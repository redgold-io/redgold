cargo build

pkill -f redgold
rm -rf ~/.rg/local_test
sleep 1
export REDGOLD_BINARY_PATH="./target/debug/redgold"
export RUST_BACKTRACE=1

export RUST_MIN_STACK=10485760

$REDGOLD_BINARY_PATH --network local --debug-id 0 --genesis >log0 2>&1 &
export NODE_1_PID=$!

sleep 7

$REDGOLD_BINARY_PATH --network local --debug-id 1 --seed-address 127.0.0.1 >log1 2>&1 &
export NODE_2_PID=$!

sleep 7

$REDGOLD_BINARY_PATH --network local --debug-id 2 --seed-address 127.0.0.1 >log2 2>&1 &
export NODE_3_PID=$!

sleep 7

cargo test local_e2e_it -- --nocapture
export TEST_EXIT_CODE=$?

script_dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )" # https://stackoverflow.com/a/246128/1826109
echo "Script dir: $script_dir"

"$script_dir/cli_test.sh" $REDGOLD_BINARY_PATH local
second_test_exit_status=$?

# Combine the exit statuses and exit the script with the combined status
final_exit_status=$((TEST_EXIT_CODE || second_test_exit_status))

cat log0; cat log1; cat log2

kill -KILL $NODE_1_PID
kill -KILL $NODE_2_PID
kill -KILL $NODE_3_PID

exit $final_exit_status
