FROM ubuntu:22.04
RUN apt update
RUN apt install -y curl
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
RUN echo 'source $HOME/.cargo/env' >> $HOME/.bashrc
ENV DEBIAN_FRONTEND=noninteractive
RUN apt install -y automake libtool libssl-dev \
 libxcb-xfixes0-dev libxcb1-dev libxcb-keysyms1-dev libpango1.0-dev libxcb-util0-dev \
 libxcb-icccm4-dev libyajl-dev libstartup-notification0-dev libxcb-randr0-dev libev-dev \
 libxcb-cursor-dev libxcb-xinerama0-dev libxcb-xkb-dev libxkbcommon-dev libxkbcommon-x11-dev \
 autoconf libxcb-xrm0 libxcb-xrm-dev automake libxcb-shape0-dev \
 g++ \
 sqlite3 libsqlite3-dev \
 nasm \
 ca-certificates \
 awscli

RUN update-ca-certificates


#RUN rustup target add x86_64-unknown-linux-gnu
#RUN rustup toolchain install stable-x86_64-unknown-linux-gnu
ADD cargo_config ~/.cargo/config
