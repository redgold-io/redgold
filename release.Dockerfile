FROM ubuntu:latest
RUN apt update
ENV DEBIAN_FRONTEND=noninteractive
RUN apt install -y automake libtool libssl-dev \
 libxcb-xfixes0-dev libxcb1-dev libxcb-keysyms1-dev libpango1.0-dev libxcb-util0-dev \
 libxcb-icccm4-dev libyajl-dev libstartup-notification0-dev libxcb-randr0-dev libev-dev \
 libxcb-cursor-dev libxcb-xinerama0-dev libxcb-xkb-dev libxkbcommon-dev libxkbcommon-x11-dev \
 autoconf libxcb-xrm0 libxcb-xrm-dev automake libxcb-shape0-dev \
 g++ \
 sqlite3 libsqlite3-dev wget

# Docker install for pulling images with auto update process.
RUN apt install -y \
            ca-certificates \
            curl \
            gnupg \
            lsb-release
RUN mkdir -p /etc/apt/keyrings
RUN curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
RUN echo \
  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
  $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null
RUN apt update
RUN apt install -y docker-ce-cli


# Get Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
RUN echo 'source $HOME/.cargo/env' >> $HOME/.bashrc
ENV PATH="/root/.cargo/bin:${PATH}"
RUN cargo install squads-multisig-cli

COPY ./redgold /redgold
RUN chmod +x /redgold
ENV REDGOLD_DOCKER=true
ENV RUST_MIN_STACK=20485760

EXPOSE 16179 16180 16181 16182 16183
ENTRYPOINT ["/redgold"]