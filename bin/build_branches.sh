#!/bin/bash

if [[ "$GITHUB_REF" =~ ^refs/heads/(predev|dev|staging|test|main)$ ]]; then
    exit 0
else
    exit 1
fi
