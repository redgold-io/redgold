#!/bin/bash
wget https://developers.yubico.com/yubikey-manager-qt/Releases/yubikey-manager-qt-1.2.5-linux.AppImage \
-O yubikey.AppImage
chmod +x yubikey.AppImage


# List of possible prefix paths
prefix_paths=("/Volumes/" "/media/$USER/")

# List of possible USB drive names (suffix paths)
suffix_paths=("Samsung USB" "NO NAME" "USB_DRIVE" "KINGSTON")

# Function to copy file to USB drive
copy_to_usb() {
    local full_path="$1"

    if [ -d "$full_path" ]; then
        echo "Copying to $full_path"
        cp redgold "$full_path/redgold"
        return 0
    fi
    return 1
}

# Try to copy to USB drive
copied=false

for prefix in "${prefix_paths[@]}"; do
    for suffix in "${suffix_paths[@]}"; do
        full_path="${prefix}${suffix}"
        if copy_to_usb "$full_path"; then
            copied=true
            break 2  # Break out of both loops
        fi
    done
done

if [ "$copied" = false ]; then
    echo "Could not find any of the specified USB drives."
fi




