HOST="$1"
rm -rf ~/.rg/dev/data_store.*;
scp root@$HOST:~/.rg/dev/data_store.sqlite ~/.rg/dev