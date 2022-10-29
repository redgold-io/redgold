

## Quickstart

This example will show the full flow with a newly generated mnemonic / keys in order to allow you to quickly test 
the network locally, for more secure operations please see further sections.

Before issuing following commands, set your current desired network environment

`main` for mainnet

`test` for testnet

`dev` for dev branch

`export REDGOLD_NETWORK='test'`

To generate a random mnemonic as a word string which writes to stdout: 

`redgold generate-words > words`

Example output:
`spray vicious advice area best device arrange federal payment remind host light cat defy soda festival song help hurt luggage police reopen myth wear wage worry egg acquire emotion occur slender wagon steel hero endless tattoo south better outside slow spice sister abandon slim surprise guide better candy`

To create a sample address from this mnemonic

`redgold --words $(cat words) address > address`

To request some sample currency for testing (default 5.0 returned)

`redgold faucet --address $(cat address) > tx_hash`

To check the faucet transaction for acceptance: 

`redgold query $(cat tx_hash)`

To send currency to someone else (create and broadcast a transaction)

`redgold --words $(cat words) --to <destination_address>`

## Secure Transactions with Trezor & Ledger

There is no support for using the Trezor / Ledger application wallets yet, however you can use the CLI automatically 
if it is installed locally. All below commands require you to have the current software installed separately and 
require additional setup before they will work. The commands will not install these dependencies for you.

[Trezor Setup Guide](https://wiki.trezor.io/Using_trezorctl_commands_with_Trezor#Install_python-trezor)

Main command from this guide that you'll need to install trezor cli:

`pip3 install trezor`

`redgold --trezor send --address <DESTINATION_ADDRESS> `

## Swaps

`redgold swap --from BTC --to RDG`