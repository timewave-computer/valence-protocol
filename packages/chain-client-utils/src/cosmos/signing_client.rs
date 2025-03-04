use bip32::{Language, Mnemonic};
use cosmos_sdk_proto::cosmos::{
    auth::v1beta1::QueryAccountInfoRequest,
    tx::v1beta1::{BroadcastMode, BroadcastTxRequest},
};
use cosmrs::{
    crypto::{secp256k1::SigningKey, PublicKey},
    tx::{self, Fee, SignDoc, SignerInfo},
    AccountId, Any,
};
use tonic::transport::Channel;

use crate::common::error::StrategistError;

use super::AuthQueryClient;

const DERIVATION_PATH: &str = "m/44'/118'/0'/0/0";

/// struct that holds any signing-related information for a cosmos-sdk client
pub struct SigningClient {
    pub signing_key: SigningKey,
    pub address: AccountId,
    pub account_number: u64,
    pub sequence: u64,
    pub chain_id: String,
    pub public_key: PublicKey,
}

impl SigningClient {
    /// builds a signing client to operate on the given channel, prefix and chain id.
    /// signs messages with the provided mnemonic.
    pub async fn from_mnemonic(
        channel: Channel,
        mnemonic: &str,
        prefix: &str,
        chain_id: &str,
    ) -> Result<Self, StrategistError> {
        let mnemonic = Mnemonic::new(mnemonic, Language::English)?;

        let seed = mnemonic.to_seed("");

        let signing_key = SigningKey::derive_from_path(seed, &DERIVATION_PATH.parse()?)?;

        let public_key = signing_key.public_key();
        let sender_account_id = public_key.account_id(prefix)?;

        let mut client = AuthQueryClient::new(channel);

        let account_info_resp = client
            .account_info(QueryAccountInfoRequest {
                address: sender_account_id.to_string(),
            })
            .await?
            .into_inner();

        let base_account = account_info_resp
            .info
            .ok_or_else(|| StrategistError::QueryError("failed to get base account".to_string()))?;

        Ok(SigningClient {
            signing_key,
            address: sender_account_id,
            account_number: base_account.account_number,
            sequence: base_account.sequence,
            chain_id: chain_id.to_string(),
            public_key,
        })
    }

    /// creates a transaction and signs it with the signing key
    pub async fn create_tx(
        &self,
        msg: Any,
        fee: Fee,
        memo: Option<&str>,
    ) -> Result<BroadcastTxRequest, StrategistError> {
        let tx_body = tx::BodyBuilder::new()
            .msg(msg)
            .memo(memo.unwrap_or_default())
            .finish();

        let auth_info =
            SignerInfo::single_direct(Some(self.public_key), self.sequence).auth_info(fee);

        let sign_doc = SignDoc::new(
            &tx_body,
            &auth_info,
            &self.chain_id.parse()?,
            self.account_number,
        )?;

        let tx_raw = sign_doc.sign(&self.signing_key)?;

        let broadcast_tx_request = BroadcastTxRequest {
            tx_bytes: tx_raw.to_bytes()?,
            mode: BroadcastMode::Sync.into(),
        };

        // TODO: in the future we can consider auto-incrementing the account sequence number here.
        // for now txs are signed with the account sequence number that gets fetched just in time.

        Ok(broadcast_tx_request)
    }
}
