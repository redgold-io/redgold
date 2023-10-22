sudo apt update
sudo apt install -y automake libtool libssl-dev
sudo apt install -y libdigest-sha3-perl
#openpnp capture
sudo apt install -y nasm
# GUI dependencies on linux, not really necessary except for bundled UI
sudo apt-get install -y libxcb-xfixes0-dev libxcb1-dev libxcb-keysyms1-dev libpango1.0-dev libxcb-util0-dev libxcb-icccm4-dev libyajl-dev libstartup-notification0-dev libxcb-randr0-dev libev-dev libxcb-cursor-dev libxcb-xinerama0-dev libxcb-xkb-dev libxkbcommon-dev libxkbcommon-x11-dev autoconf libxcb-xrm0 libxcb-xrm-dev automake libxcb-shape0-dev

#rustup default nightly