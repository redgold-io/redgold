#!/bin/bash

script_dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
echo "Script dir: $script_dir"

bump_patch_version() {
  # Parse version
  local version=$("$script_dir/get_version.sh")
  local major=$(echo $version | awk -F. '{print $1}')
  local minor=$(echo $version | awk -F. '{print $2}')
  local patch=$(echo $version | awk -F. '{print $3}')

  # Bump patch version
  patch=$((patch + 1))

  # Reconstruct version
  local new_version="${major}.${minor}.${patch}"

  echo "$new_version $(pwd)"

  # Replace the version in the top level 3rd line of file
  sed -i.bak "3s/version = \".*\"/version = \"$new_version\"/" Cargo.toml && rm Cargo.toml.bak

  # Replace the redgold module versions
  sed -i.bak "/^redgold-/s/version = \".*\"/version = \"$new_version\"/" Cargo.toml && rm Cargo.toml.bak
}

set -e

bump_patch_version
#
#for dir in keys sdk sdk-client; do
#  (cd "$dir" && bump_patch_version)
#done