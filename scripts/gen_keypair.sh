#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
# Need to build binary first
set -e

env=${1:-debug}

echo "*** Generating keypair ***"

./target/$env/climacoin-node key generate