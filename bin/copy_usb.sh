#!/bin/bash
wget https://github.com/redgold-io/redgold/releases/download/release%2Fdev/redgold_linux \
-O redgold
chmod +x redgold
# TODO: Pick up usb drive with bash?
#cp redgold /Volumes/Samsung\ USB/redgold
#cp redgold /Volumes/NO\ NAME/redgold

# Check if the first directory exists and copy there, otherwise check the second directory
if [ -d "/Volumes/Samsung USB" ]; then
    cp redgold "/Volumes/Samsung USB/redgold"
elif [ -d "/Volumes/NO NAME" ]; then
    cp redgold "/Volumes/NO NAME/redgold"
else
    echo "Neither directory exists!"
fi






