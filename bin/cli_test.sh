
REDGOLD_BINARY_PATH=$1
network=$2

export REDGOLD_NETWORK="$network"

set -e

$REDGOLD_BINARY_PATH generate-words > words

echo "Generated test words: $(cat words)"

export REDGOLD_WORDS="$(cat words)"

$REDGOLD_BINARY_PATH address --index 0 > address

echo "Generated test address: $(cat address)"

$REDGOLD_BINARY_PATH faucet --to "$(cat address)" > faucet_tx_hash

echo "Faucet tx hash: $(cat faucet_tx_hash)"

$REDGOLD_BINARY_PATH query --hash "$(cat faucet_tx_hash)"

echo "Done query for faucet_tx_hash"

$REDGOLD_BINARY_PATH address --index 1 > address2

echo "Address 2: $(cat address2)"

$REDGOLD_BINARY_PATH send --to "$(cat address2)" --amount 2.0 > send_tx_hash

echo "Send tx hash: $(cat send_tx_hash)"

sleep 20

$REDGOLD_BINARY_PATH query --hash "$(cat send_tx_hash)"

echo "Done query for send_tx_hash"

float_value=$($REDGOLD_BINARY_PATH balance --address "$(cat address2)")

echo "Final balance of address2: $float_value"

# TODO: Drain transaction back to faucet

# TODO: Re-enable test after fixing bug:

#float_value="2.0"

# Check if the output is a valid float
if [[ ! $float_value =~ ^-?[0-9]+(\.[0-9]+)?$ ]]; then
    echo "Error: invalid float value"
    exit 1
fi

# Compare the float value using awk
awk -v value="$float_value" 'BEGIN { if (value <= 1) { exit 1; } }'

# Check the exit status of the comparison
comparison_exit_status=$?

# If the exit status is non-zero, the value is less than or equal to 1
if [ $comparison_exit_status -ne 0 ]; then
    echo "Error: value is less than or equal to 1"
    exit 1
fi
