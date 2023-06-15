
Sqlx macro compilation unfortunately requires an environment variable to be set before use.

`export DATABASE_URL=sqlite://$HOME/.rg/sqlx/data_store.sqlite`

Please set this or the project will not compile. An example is seen in .env in top level directory

Note if changing the database schema, you'll need to explicitly recompile data module to run migrations. 
A `cargo build` in terminal should fix it.

Start by installing rust with 

`curl https://sh.rustup.rs -sSf | sh -s -- -y`

This project (sometimes) requires rust nightly (locally on some machines but not on CI) due to libp2p issues. Enable it with:

`rustup install nightly`
`rustup override set nightly`

# IntelliJ

IntelliJ rust plugin seems to be a lot better than VSCode namely due to latency. IJ lets you keep typing and 
compiles in the background, while every time you auto-save in VSCode it will initiate a compile at a small delay 
and break all the auto-complete while it's recompiling. When more modules are added to prevent excess recompilation 
in this project, the latency would probably be better, but IJ is a lot more friendly right now.

For protobuf code gen to autocomplete properly, you need to enable `org.rust.cargo.evaluate.build.scripts`. 
From the UI do this using Actions -> Experimental Features -> look for `org.rust.cargo.evaluate.build.scripts` 
and enable it. This setting can also be found on disk locally under a path like:
`/Users/$USER/Library/Application Support/JetBrains/CLion2021.2/options/other.xml:46:`

If using JetBrains Gateway then you'll need to find the file on disk on the remote machine:

`user@main:~/projects$ nano ~/.config/JetBrains/RemoteDev-CL/_home_$USER_projects_redgold-core/options/other.xml`

And add a line like 

`"experimentalFeature.org.rust.cargo.evaluate.build.scripts": "true"`

See [here](https://jen20.dev/post/completion-of-generated-code-in-intellij-rust/) for a longer explanation (outdated)

If using Gateway, make sure you have updated the .profile on the remote machine with DATABASE_URL.

# VSCode 

Useful for remote development, more stable than Jetbrains Gateway
https://hands-on-rust.com/2022/03/26/working-with-rust-on-a-remote-linux-server-with-rust-analyzer/

Make sure you update the .profile on the remote server BEFORE starting vs code server
otherwise pkill -f .vscode-server

# WASM

`rustup target add wasm32-unknown-unknown`
`rustup target add wasm32-wasi`

# Mac

For WASM compilation errors on M1 Mac, try this:

```shell
softwareupdate --install-rosetta
export MACOSX_DEPLOYMENT_TARGET=10.7
```

https://github.com/rust-bitcoin/rust-secp256k1/issues/283

https://github.com/rustwasm/wasm-pack/issues/952


If you get an error with -lgmp during build, make sure to add the following to your .profile or .zshrc
(This is a requirement from the multiparty threshold ECDSA library)

```shell
export LIBRARY_PATH=$LIBRARY_PATH:/opt/homebrew/lib
export INCLUDE_PATH=$INCLUDE_PATH:/opt/homebrew/include
```


This may be out of date, but at least some commands here are probably required
```shell
brew update
# for protobuf generation
brew install automake
brew install libtool
```

Optional install grafana for local monitoring: 
```
brew install grafana
brew services start grafana
```

Note: this section is incomplete.

Cross compiling from mac to [Linux](https://stackoverflow.com/questions/41761485/how-to-cross-compile-from-mac-to-linux) from Mac:

`rustup target add x86_64-unknown-linux-gnu`

`cargo install cross`

`rustup toolchain install stable-x86_64-unknown-linux-gnu`

`docker build -t linux_build .`

`cross build --release --target=x86_64-unknown-linux-gnu`

# Linux

Linux LLVM recommended [install](https://apt.llvm.org/) but this might have issues? 
```shell
wget https://apt.llvm.org/llvm.sh
chmod +x llvm.sh
sudo ./llvm.sh <version number>
```

`12` is recommended current version.

Alternative 
```shell
sudo apt install llvm llvm-12-dev
```

Build dependencies (see Dockerfile as well)
```shell
sudo apt install -y automake libtool libssl-dev \
libxcb-xfixes0-dev libxcb1-dev libxcb-keysyms1-dev libpango1.0-dev libxcb-util0-dev \
libxcb-icccm4-dev libyajl-dev libstartup-notification0-dev libxcb-randr0-dev libev-dev \
libxcb-cursor-dev libxcb-xinerama0-dev libxcb-xkb-dev libxkbcommon-dev libxkbcommon-x11-dev \
autoconf libxcb-xrm0 libxcb-xrm-dev automake libxcb-shape0-dev \
g++ \
sqlite3 libsqlite3-dev
```

Cross compiling to mac: 

`cargo install cross --git https://github.com/cross-rs/cross`

`rustup target add x86_64-apple-darwin`

`cross build --target x86_64-apple-darwin`

^ This step doesn't work, need a proper solution here involving osxcross. This
https://github.com/Shogan/rust-musl-action/blob/master/Dockerfile is the closest example but fails with a GLIBC error
Possible to fix by updating mac_cross.Dockerfile

Alternative here: https://wapl.es/rust/2019/02/17/rust-cross-compile-linux-to-macos.html

# Misc

For capturing println statements live

`cargo test -- --nocapture`

Recommend setting git settings:

`git config --global user.email "you@example.com"`
`git config --global user.name "Your Name"`

Or in ~/.gitconfig (full settings with GPG)

```
[user]
        name = your_name
        email = your_email
        signingkey = your_gpg_key_id
[github]
        user = your_github_username
        token = your_token
[commit]
        gpgsign = true
```

Cargo publishing: https://crates.io/settings/tokens

# Errors:

Make sure you're on most recent rustc version. If you get an error like this:

```
error: package `wasmtime v8.0.1` cannot be built because it requires rustc 1.66.0 or newer, while the currently active rustc version is 1.65.0
```

Update rustc with

`rustup update`

If you have dependency conflicts or issues please first use:

`cargo update`

`cargo clean`