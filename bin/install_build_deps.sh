sudo apt update
# libssl is used for embedded SSH server management
sudo apt install -y automake libtool libssl-dev
# Used for shasum cli tools
sudo apt install -y libdigest-sha3-perl
#openpnp capture
sudo apt install -y nasm
# Not required on all ubuntu images, but some servers need this to build
sudo apt-get install -y llvm libclang-dev cmake libjpeg-turbo8-dev #libjpeg is for openpnp-capture
# GUI dependencies on linux, not really necessary except for bundled UI
sudo apt-get install -y libxcb-xfixes0-dev libxcb1-dev libxcb-keysyms1-dev libpango1.0-dev libxcb-util0-dev libxcb-icccm4-dev libyajl-dev libstartup-notification0-dev libxcb-randr0-dev libev-dev libxcb-cursor-dev libxcb-xinerama0-dev libxcb-xkb-dev libxkbcommon-dev libxkbcommon-x11-dev autoconf libxcb-xrm0 libxcb-xrm-dev automake libxcb-shape0-dev

# multiparty threshold math
sudo apt-get install -y libgmp3-dev lld

#rustup default nightly

# fuse support
sudo apt-get install -y fuse3 libfuse3-dev libfuse-dev pkg-config

# openpgp support, only required for airgap linux
sudo apt install -y clang llvm pkg-config nettle-dev
