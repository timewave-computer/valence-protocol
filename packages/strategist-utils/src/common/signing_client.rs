use bip32::{Language, Mnemonic};
use cosmos_sdk_proto::cosmos::{
    auth::v1beta1::QueryAccountInfoRequest,
    tx::v1beta1::{BroadcastMode, BroadcastTxRequest, BroadcastTxResponse},
};
use cosmrs::{
    crypto::{secp256k1::SigningKey, PublicKey},
    tx::{self, Fee, Msg, Raw, SignDoc, SignerInfo},
    AccountId, Any, Coin,
};
use tonic::transport::Channel;

use super::error::StrategistError;

pub struct SigningClient {
    pub signing_key: SigningKey,
    pub address: AccountId,
    pub account_number: u64,
    pub sequence: u64,
    pub chain_id: String,
    pub public_key: PublicKey,
}

const DERIVATION_PATH: &str = "m/44'/118'/0'/0/0";

impl SigningClient {
    pub async fn from_mnemonic(
        channel: Channel,
        mnemonic: &str,
        prefix: &str,
        chain_id: &str,
    ) -> Result<Self, StrategistError> {
        let mnemonic = Mnemonic::new(mnemonic, Language::English)
            .map_err(|e| StrategistError::ParseError(e.to_string()))?;

        let seed = mnemonic.to_seed("");

        let signing_key = SigningKey::derive_from_path(seed, &DERIVATION_PATH.parse().unwrap())
            .map_err(|e| StrategistError::ParseError(e.to_string()))?;

        let public_key = signing_key.public_key();
        let sender_account_id = public_key.account_id(prefix).unwrap();

        let mut client =
            cosmos_sdk_proto::cosmos::auth::v1beta1::query_client::QueryClient::new(channel);

        let account_info_resp = client
            .account_info(QueryAccountInfoRequest {
                address: sender_account_id.to_string(),
            })
            .await
            .map_err(|e| StrategistError::QueryError(e.to_string()))?
            .into_inner();

        let base_account = match account_info_resp.info {
            Some(base_acc) => base_acc,
            None => {
                return Err(StrategistError::QueryError(
                    "failed to get base account".to_string(),
                ))
            }
        };

        Ok(SigningClient {
            signing_key,
            address: sender_account_id,
            account_number: base_account.account_number,
            sequence: base_account.sequence,
            chain_id: chain_id.to_string(),
            public_key,
        })
    }

    pub async fn create_tx(&self, msg: Any) -> Result<BroadcastTxRequest, StrategistError> {
        let fee = Fee::from_amount_and_gas(
            Coin {
                denom: "untrn".parse().unwrap(),
                amount: 100_000,
            },
            100_000u64,
        );

        let tx_body = tx::BodyBuilder::new().msg(msg).memo("test memo").finish();

        let auth_info =
            SignerInfo::single_direct(Some(self.public_key), self.sequence).auth_info(fee);

        let sign_doc = SignDoc::new(
            &tx_body,
            &auth_info,
            &self.chain_id.parse().unwrap(),
            self.account_number,
        )
        .unwrap();

        let tx_raw = sign_doc.sign(&self.signing_key).unwrap();

        let broadcast_tx_request = BroadcastTxRequest {
            tx_bytes: tx_raw.to_bytes().unwrap(),
            mode: BroadcastMode::Sync.into(),
        };

        Ok(broadcast_tx_request)
    }

    // pub async fn broadcast_tx() -> Result<BroadcastTxResponse, StrategistError> {
    //     let request = BroadcastTxRequest {
    //         tx_bytes: ,
    //         mode: BroadcastMode::Block.into(),
    //     };
    // }
}
