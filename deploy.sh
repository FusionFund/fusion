#!/bin/sh

./build.sh

if [ $? -ne 0 ]; then
  echo ">> Error building contract"
  exit 1
fi

echo ">> Deploying contract"

# https://docs.near.org/tools/near-cli#near-dev-deploy
# rm -rf neardev &&
near contract deploy fusionfund.testnet use-file ./target/wasm32-unknown-unknown/release/fund.wasm without-init-call network-config testnet sign-with-keychain send

# near dev-deploy ./target/wasm32-unknown-unknown/release/contract.wasm
# steps to crate account
# near create-account <accountId> --useFaucet
# if it couldn't save password to keychain, then sudo apt install gnome-keyring