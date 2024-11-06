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

TESTNET_INFO="--node https://neutron-testnet-rpc.polkachu.com:443 --chain-id pion-1"
LOCAL_IC_INFO="--node http://0.0.0.0:45791 --chain-id localneutron-1"

TESTNET_EXECUTE_FLAGS="--gas-prices $GAS_PRICES --gas auto --gas-adjustment 1.5 --output json $TESTNET_INFO -y"
LOCAL_IC_EXECUTE_FLAGS="--gas-prices $GAS_PRICES --gas auto --gas-adjustment 1.5 --output json $LOCAL_IC_INFO -y"

ARTIFACTS_PATH="../artifacts"

# File names
REGISTRY_FILE_NAME="$ARTIFACTS_PATH/valence_workflow_registry.wasm"
AUTH_FILE_NAME="$ARTIFACTS_PATH/valence_authorization.wasm"
PROCESSOR_FILE_NAME="$ARTIFACTS_PATH/valence_processor.wasm"
BASE_ACCOUNT_FILE_NAME="$ARTIFACTS_PATH/valence_base_account.wasm"
FORWARDER_FILE_NAME="$ARTIFACTS_PATH/valence_forwarder_library.wasm"

if [[ "$COMMAND" == 'registry' ]]; then
  $BINARY tx wasm s $REGISTRY_FILE_NAME --from $OWNER_ADDR $TESTNET_EXECUTE_FLAGS
elif [[ "$COMMAND" == 'auth' ]]; then
  $BINARY tx wasm s $AUTH_FILE_NAME --from $OWNER_ADDR $TESTNET_EXECUTE_FLAGS
elif [[ "$COMMAND" == 'processor' ]]; then
  $BINARY tx wasm s $PROCESSOR_FILE_NAME --from $OWNER_ADDR $TESTNET_EXECUTE_FLAGS
elif [[ "$COMMAND" == 'base_account' ]]; then
  $BINARY tx wasm s $BASE_ACCOUNT_FILE_NAME --from $OWNER_ADDR $TESTNET_EXECUTE_FLAGS
elif [[ "$COMMAND" == 'forwarder' ]]; then
  $BINARY tx wasm s $FORWARDER_FILE_NAME --from $OWNER_ADDR $TESTNET_EXECUTE_FLAGS
else
  echo "Unknown command"
fi
