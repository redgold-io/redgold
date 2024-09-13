#!/bin/bash

# Set environment variables (these should be provided when running the script)
# GITHUB_TOKEN, TOOLCHAIN, VERSION, SOURCE_BRANCH, DESTINATION_BRANCH

# Function to check if branches are different
check_branches() {
    git fetch origin "$SOURCE_BRANCH" "$DESTINATION_BRANCH"
    SOURCE_HASH=$(git rev-parse "origin/$SOURCE_BRANCH")
    DESTINATION_HASH=$(git rev-parse "origin/$DESTINATION_BRANCH")
    if [ "$SOURCE_HASH" = "$DESTINATION_HASH" ]; then
        echo "$SOURCE_BRANCH and $DESTINATION_BRANCH branches are already at the same commit. No action needed."
        return 1
    else
        echo "$SOURCE_BRANCH and $DESTINATION_BRANCH branches are different. Proceeding with release process."
        return 0
    fi
}

# Main release rollout process
release_rollout() {
    # Checkout code
    git checkout "$SOURCE_BRANCH"

    # Run version bump script, but only if source_branch is dev
    if [ "$SOURCE_BRANCH" = "dev" ]; then
        echo "Running version bump script"
        chmod +x ./bin/bump_versions.sh
        ./bin/bump_versions.sh

        git add .
        git commit -m "[skip ci] Bump versions for release" || echo "No changes to commit"

        # Push to source branch
        git push origin "$SOURCE_BRANCH"
        echo "Updated version"

    fi

    # Set version
    VERSION=$(head Cargo.toml | grep 'version = ' | cut -d "=" -f 2 | tr -d ' "')
    echo "Releasing version $VERSION from $SOURCE_BRANCH to $DESTINATION_BRANCH"

    # Push to destination branch
    git push origin "$SOURCE_BRANCH":"$DESTINATION_BRANCH"
    echo "Pushed to $DESTINATION_BRANCH, now running release test"

    # Run tests
    export RUSTFLAGS="-C link-arg=-fuse-ld=lld"
    ./bin/release_test.sh
}

# Main script execution
if [ -z "$SOURCE_BRANCH" ] || [ -z "$DESTINATION_BRANCH" ]; then
    echo "Error: SOURCE_BRANCH and DESTINATION_BRANCH must be set"
    exit 1
fi

export REDGOLD_NETWORK=DESTINATION_BRANCH

if check_branches; then
    release_rollout
else
    echo "No release needed. Exiting."
    exit 0
fi