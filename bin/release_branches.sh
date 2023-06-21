#!/bin/bash

if [[ "$GITHUB_REF" =~ ^refs/heads/(staging|test|main)$ ]]; then
    echo "MATCH=1"
else
    echo "MATCH=0"
fi