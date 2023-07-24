#!/bin/bash


script_dir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )" # https://stackoverflow.com/a/246128/1826109
echo "Script dir: $script_dir"


bump_patch_version() {
  # Parse version
  local version=$("$script_dir/get_version.sh")
  local major=$(echo $version | cut -d. -f1)
  local minor=$(echo $version | cut -d. -f2)
  local patch=$(echo $version | cut -d. -f3)

  # Bump patch version
  patch=$((patch + 1))

  # Reconstruct version
  local new_version="${major}.${minor}.${patch}"

  echo "$new_version $(pwd)"

  # Replace the version in the top level 3rd line of file
  sed -i '' "3s/version = \".*\"/version = \"$new_version\"/" Cargo.toml

  # Replace the redgold module versions
  sed -i '' "/^redgold-/s/version = \".*\"/version = \"$new_version\"/" Cargo.toml

}


set -e

bump_patch_version

cd schema
bump_patch_version
cd ..

cd data
bump_patch_version
cd ..

cd executor
bump_patch_version
cd ..
