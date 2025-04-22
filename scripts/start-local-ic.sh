#!/usr/bin/env bash
cd e2e

CHAIN_CONFIG=$1
MAX_ATTEMPTS=10
ATTEMPT=1
SUCCESS=false

# Determine the path to the local-ic binary
LOCAL_IC_BIN=${LOCAL_IC_BIN:-}

if [[ -z "$LOCAL_IC_BIN" ]]; then
  if command -v local-ic &>/dev/null; then
    LOCAL_IC_BIN=$(command -v local-ic)
  elif [[ -x "/tmp/local-ic" ]]; then
    LOCAL_IC_BIN="/tmp/local-ic"
  else
    echo "Error: local-ic binary not found in PATH or /tmp. Please set LOCAL_IC_BIN environment variable."
    exit 1
  fi
fi

while [[ "$SUCCESS" = false && "$ATTEMPT" -lt "$MAX_ATTEMPTS" ]]; do
  "$LOCAL_IC_BIN" start $CHAIN_CONFIG --api-port 42069 &
  curl --head -X GET --retry 200 --retry-connrefused --retry-delay 5 http://localhost:42069
  echo "$(date): Successfully queried Local-IC"
  sleep 20
  # Check to see if chain config has been created which is the last step of local-ic startup
  CHAIN_INFO=$(<configs/logs.json)
  if [[ "$CHAIN_INFO" != "{}" ]]; then
    echo "$(date): Local-IC chain info created indicating successful startup"
    SUCCESS=true
  else
    echo "$(date) (Attempt $ATTEMPT): Local-IC failed to start, trying again"
    ((ATTEMPT++))
    pkill local-ic # Cleanup background local-ic process
  fi
done

if [[ "$SUCCESS" = false ]]; then
  echo "$(date): Exceeded maximum number of attempts to start Local-IC"
fi
