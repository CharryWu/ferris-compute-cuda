#!/bin/sh
# Configures git to use the project's .githooks directory for commit hooks.
# Run once after cloning: ./scripts/setup-git-hooks.sh

git config core.hooksPath .githooks
echo "Git hooks configured. Pre-commit will run client and host tests."
