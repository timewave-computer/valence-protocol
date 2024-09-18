#!/bin/bash

CHAIN=$1
shift
COMMAND=$1
shift

if [[ "$CHAIN" == 'neutron' || "$CHAIN" == 'ntrn' ]]; then
  BINARY="neutrond"
  GAS_PRICES="0.055untrn"
  OWNER_ADDR="neutron1tl0w0djc5y53aqfr60a794f02drwktpujm5xxe"
  ADMIN_ADDR="neutron1tl0w0djc5y53aqfr60a794f02drwktpujm5xxe"

  CODE_ID_REGISTRY=6774

else
  echo "Unknown chain"
fi

TESTNET_NODE="https://neutron-testnet-rpc.polkachu.com:443"
TESTNET_CHAIN_ID="pion-1"

EXECUTE_FLAGS="--gas-prices $GAS_PRICES --gas auto --gas-adjustment 1.5 --output json --node $TESTNET_NODE --chain-id $TESTNET_CHAIN_ID -y"

################################################
################### Registry ###################
################################################
if [[ "$COMMAND" == 'registry' ]]; then
  init_msg=$(jq -n \
    --arg admin "$ADMIN_ADDR" \
    '{
      admin: $admin
    }')

  $BINARY tx wasm init $CODE_ID_REGISTRY "$init_msg" --label "Valence workflow registry" \
    --admin $OWNER_ADDR --from $OWNER_ADDR $EXECUTE_FLAGS

else
  echo "Unknown command"
fi
