
NOTE: This is WIP and not finished. Not all commands working / complete yet

There are many variations on setting up a node, but this is the recommended flow. If you know what you are doing, 
feel free to customize this as much as possible to your desired security setting, as some steps can be omitted or 
changed depending on whether you're using a cold computer or desire more or less security.


Use the deploy wizard 

```shell
redgold deploy
```

Or do each step manually here with: 

Set up an environment variable with a cryptomator cloud-backed up drive:

```shell
export REDGOLD_SECURE_DATA_PATH="your_backup"
```

Generate a purely random mnemonic seed for storage

```shell
redgold generate-mnemonic --random-seed-backup
```


redgold add-server -h hostnoc.redgold.io

```shell
docker run redgoldio/redgold:dev
```