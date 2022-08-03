# Validator Setup

1. Clone the repo and build binary by running ```cargo build --release```
2. Generate node key ```./target/release/climacoin-node generate-node-key```
3. Run the node using the specRaw.json of testnet/mainnet from chainspec directory.
```
    ./target/release/climacoin-node \
    --base-path /tmp/climacoin/node01  \
    --chain chainspec/< testnet or mainnet >/specRaw.json \
    --port 30334 \
    --ws-port 9944 \
    --rpc-port 9934 \
    --validator \
    --node-key < insert node key here > \
    --rpc-external --rpc-cors all --rpc-methods=Unsafe --pruning archive --name node1
```
Note: Unsafe workaround needs to be found
3. Generate private and public key pair.
```./target/release/climacoin-node key generate --scheme Sr25519```
4. Insert the keys to the node.
```
    ./target/release/climacoin-node key insert \
    --base-path /tmp/climacoin/node01 \
    --chain chainspec/< testnet or mainnet >/specRaw.json \
    --scheme Sr25519 \
    --suri "< insert seed here >"  \
    --key-type babe
```
```
    ./target/release/climacoin-node key insert \
    --base-path /tmp/climacoin/node01 \
    --chain chainspec/< testnet or mainnet >/specRaw.json \
    --scheme Ed25519 \
    --suri "< insert seed here >"  \
    --key-type gran
```
```
    ./target/release/climacoin-node key insert \
    --base-path /tmp/climacoin/node01 \
    --chain chainspec/< testnet or mainnet >/specRaw.json \
    --scheme Sr25519 \
    --suri "< insert seed here >"  \
    --key-type imon
```
5. For nginx configuration. Refer [this](nginx-setup.md).
6. Connect to the node using Polkadot apps. [Click here](https://polkadot.js.org/apps/#/explorer) Switch to the node's Ip or domain.
6. Go to Developer > RPC Calls. Author > rotatekeys, copy the key.
7. Go to Network > Staking. Then Accounts > Add Validator. Sign and submit with stash/controller account, staking amount and paste the keys.
8. With New Era, validator should validating.