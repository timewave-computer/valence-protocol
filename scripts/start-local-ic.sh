#!/usr/bin/env bash
cd local-interchaintest

CHAIN_CONFIG=$1
MAX_ATTEMPTS=10
ATTEMPT=1
SUCCESS=false

while [[ "$SUCCESS" = false && "$ATTEMPT" -lt "$MAX_ATTEMPTS" ]]; do
  /tmp/local-ic start $CHAIN_CONFIG --api-port 42069 &
  curl --head -X GET --retry 200 --retry-connrefused --retry-delay 5 http://localhost:42069
  echo "$(date): Successfully queried Local-IC"
  sleep 10
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
