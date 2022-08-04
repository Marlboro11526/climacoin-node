#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
# Pass base seed, spec path and base path respectively as arguments
set -e

seed=$1
spec=${2:-chainspec/testnet/specRaw.json}
path=${3:-/tmp/climacoin/node01}

echo "*** Inserting keys ***"
echo ""
echo "seed: $seed"
echo "spec: $spec"
echo "path: $path"
echo ""

./target/release/climacoin-node key insert \
  --base-path "$path" \
  --chain "$spec" \
  --scheme Sr25519 \
  --suri "$seed"  \
  --key-type babe

./target/release/climacoin-node key insert \
  --base-path "$path" \
  --chain "$spec" \
  --scheme Ed25519 \
  --suri "$seed"  \
  --key-type gran

  ./target/release/climacoin-node key insert \
  --base-path "$path" \
  --chain "$spec" \
  --scheme Sr25519 \
  --suri "$seed"  \
  --key-type imon

echo "Added babe, gran and imon keys"
