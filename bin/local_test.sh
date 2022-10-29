
export REDGOLD_BINARY_PATH="./target/debug/redgold"

$REDGOLD_BINARY_PATH --network local --debug_id 0

mycommand &
last_pid=$!
sleep( $RANDOM )
kill -KILL $last_pid
