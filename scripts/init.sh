#!/bin/bash

CHAIN=$1
shift
COMMAND=$1
shift

if [[ "$CHAIN" == 'neutron' || "$CHAIN" == 'ntrn' ]]; then
  BINARY="neutrond"
  GAS_PRICES="0.1untrn"
  OWNER_ADDR="neutron1tl0w0djc5y53aqfr60a794f02drwktpujm5xxe"
  ADMIN_ADDR="neutron1tl0w0djc5y53aqfr60a794f02drwktpujm5xxe"

  CODE_ID_REGISTRY=6774

else
  echo "Unknown chain"
fi

TESTNET_INFO="--node https://neutron-testnet-rpc.polkachu.com:443 --chain-id pion-1"
LOCAL_IC_INFO="--node http://0.0.0.0:45791 --chain-id localneutron-1"

TESTNET_EXECUTE_FLAGS="--gas-prices $GAS_PRICES --gas auto --gas-adjustment 1.5 --output json $TESTNET_INFO -y"
LOCAL_IC_EXECUTE_FLAGS="--gas-prices $GAS_PRICES --gas auto --gas-adjustment 1.5 --output json $LOCAL_IC_INFO -y"

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
    --admin $OWNER_ADDR --from $OWNER_ADDR $TESTNET_EXECUTE_FLAGS

else
  echo "Unknown command"
fi
