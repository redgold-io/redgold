FROM ubuntu:latest
RUN apt update
ENV DEBIAN_FRONTEND=noninteractive
RUN apt install -y automake libtool libssl-dev \
 libxcb-xfixes0-dev libxcb1-dev libxcb-keysyms1-dev libpango1.0-dev libxcb-util0-dev \
 libxcb-icccm4-dev libyajl-dev libstartup-notification0-dev libxcb-randr0-dev libev-dev \
 libxcb-cursor-dev libxcb-xinerama0-dev libxcb-xkb-dev libxkbcommon-dev libxkbcommon-x11-dev \
 autoconf libxcb-xrm0 libxcb-xrm-dev automake libxcb-shape0-dev \
 g++ \
 sqlite3 libsqlite3-dev
ADD ./redgold ./redgold
ENV REDGOLD_DOCKER=true
CMD ["./redgold"]