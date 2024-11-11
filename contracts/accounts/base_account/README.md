# Valence Base Account

The **Valence Base Account** is the main type of account used by Valence programs. It suports an **admin** and a set of **approved services** that can execute arbitrary messages on behalf of the account.
A typical use case is for a service, configured as approved service on a given account, to transfer part (or the entirety) of the account's token balance (native or CW20) to a target recipient.
