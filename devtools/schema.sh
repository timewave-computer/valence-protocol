#!/usr/bin/env bash

CONTRACTS_DIR=$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )/../contracts
echo $CONTRACTS_DIR

# Generate the schema files and remove the raw folder
# arg is the path inside the contract folder
do_schema () {
    cd "$CONTRACTS_DIR/$1" || { echo "No $1 dir" ; exit 1; }
    cargo schema || { echo "Failed doing schema $1" ; exit 1; }
    rm -r schema/raw
}

# Schema for account
do_schema "accounts/base_account"
