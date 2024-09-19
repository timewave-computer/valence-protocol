#!/bin/bash

CHAIN=$1
shift
COMMAND=$1
shift
INIT_BY=$1
shift

if [[ "$CHAIN" == 'neutron' || "$CHAIN" == 'ntrn' ]]; then
  BINARY="neutrond"
  GAS_PRICES="0.055untrn"
  OWNER_ADDR="neutron1tl0w0djc5y53aqfr60a794f02drwktpujm5xxe"

# elif [[ "$CHAIN" == 'juno' ]]; then
#   BINARY="junod"
#   GAS_PRICES="0.025ujunox"
#   OWNER_ADDR="juno17s47ltx2hth9w5hntncv70kvyygvg0qr83zghn"

else
  echo "Unknown chain"
fi

if [ -z "$INIT_BY" ]; then
  ADDRESSES="$OWNER_ADDR"
else
  ADDRESSES="$OWNER_ADDR,$AUCTIONS_MANAGER_ADDR,$INIT_BY"
fi

TESTNET_NODE="https://neutron-testnet-rpc.polkachu.com:443"
TESTNET_CHAIN_ID="pion-1"

EXECUTE_FLAGS="--gas-prices $GAS_PRICES --gas auto --gas-adjustment 1.5 --output json --instantiate-anyof-addresses $ADDRESSES --node $TESTNET_NODE --chain-id $TESTNET_CHAIN_ID -y"
ACCOUNT_EXECUTE_FLAGS="--gas-prices $GAS_PRICES --gas auto --gas-adjustment 1.5 --output json -y"
ARTIFACTS_PATH="../artifacts"

# File names
REGISTRY_FILE_NAME="$ARTIFACTS_PATH/valence_workflow_registry.wasm"

if [[ "$COMMAND" == 'registry' ]]; then
  $BINARY tx wasm s $REGISTRY_FILE_NAME --from $OWNER_ADDR $EXECUTE_FLAGS
else
  echo "Unknown command"
fi
