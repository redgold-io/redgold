#!/bin/bash

# Array of branch names
branches=("dev" "main" "staging" "test")

# Current branch
current_branch=$(git rev-parse --abbrev-ref HEAD)

# Function to push to a branch
push_to_branch() {
    local branch=$1
    echo "Pushing to $branch..."
    git push origin $current_branch:$branch
    if [ $? -eq 0 ]; then
        echo "Successfully pushed to $branch"
    else
        echo "Failed to push to $branch"
    fi
}

# Main script
echo "Starting multi-branch push..."

# Loop through branches and push
for branch in "${branches[@]}"; do
    push_to_branch $branch
done

echo "Multi-branch push completed."