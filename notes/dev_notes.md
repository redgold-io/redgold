


https://wiki.trezor.io/Using_trezorctl_commands_with_Trezor#Install_python-trezor

trezorctl get-address -n "m/49'/0'/0'/0/0" -t "p2shsegwit" -d
trezorctl sign-message -n "m/44'/0'/0'/0/0" "ahoy"



curl -X POST http://localhost:16181/request \
-H 'Content-Type: application/json' \
-d '{"about_node_request": {"verbose": true}}'




curl -X POST http://lb.redgold.io:16181/request \
-H 'Content-Type: application/json' \
-d '{"about_node_request": {"verbose": true}}'


