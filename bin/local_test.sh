

cargo build

pkill -f redgold
rm -rf ~/.rg/local_test
sleep 1
export REDGOLD_BINARY_PATH="./target/debug/redgold"
export RUST_BACKTRACE=1

export RUST_MIN_STACK=10485760


$REDGOLD_BINARY_PATH --network local --debug-id 0 --genesis node >log0 2>&1 &
export NODE_1_PID=$!

sleep 7

$REDGOLD_BINARY_PATH --network local --debug-id 1 --seed-address 127.0.0.1 node >log1 2>&1 &
export NODE_2_PID=$!

sleep 7

$REDGOLD_BINARY_PATH --network local --debug-id 2 --seed-address 127.0.0.1 node >log2 2>&1 &
export NODE_3_PID=$!

sleep 7

cleanup() {
    kill -KILL $NODE_1_PID
    kill -KILL $NODE_2_PID
    kill -KILL $NODE_3_PID

    cat log0; cat log1; cat log2
}

trap cleanup EXIT

cargo test local_e2e_it -- --nocapture || { echo 'First test failed, aborting.'; exit 1; }
export TEST_EXIT_CODE=$?

# If this exit code not 0 then kill processes and abort remainder

script_dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )" # https://stackoverflow.com/a/246128/1826109
echo "Script dir: $script_dir"

"$script_dir/cli_test.sh" $REDGOLD_BINARY_PATH local
second_test_exit_status=$?

exit $second_test_exit_status
