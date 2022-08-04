#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

env=${1:-debug}

echo "*** Starting Node in $env mode ***"
./target/$env/climacoin-node --dev --tmp
