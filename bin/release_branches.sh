#!/bin/bash

if [[ "$GITHUB_REF" =~ ^refs/heads/(staging|test|main)$ ]]; then
    echo "match=1" >> "$GITHUB_OUTPUT"
else
    echo "match=0" >> "$GITHUB_OUTPUT"
fi