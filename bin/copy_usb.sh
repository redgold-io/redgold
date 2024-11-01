#!/bin/bash
wget https://github.com/redgold-io/redgold/releases/download/release%2Fdev/redgold_linux_ubuntu20 \
-O redgold
chmod +x redgold


# List of possible prefix paths
prefix_paths=("/Volumes/" "/media/$USER/")

# List of possible USB drive names (suffix paths)
suffix_paths=("Samsung USB" "NO NAME" "USB_DRIVE" "KINGSTON", "B2CF-75D8")

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




