use bech32::{encode, primitives::decode::CheckedHrpstring, Bech32, Hrp};
use cosmwasm_std::{
    Addr, Api, CanonicalAddr, HashFunction, RecoverPubkeyError, StdError, StdResult,
    VerificationError,
};
use rand_core::OsRng;

pub struct MockApi {
    bech32_prefix: String,
}

impl MockApi {
    pub fn new(bech32_prefix: String) -> Self {
        MockApi { bech32_prefix }
    }
}

impl Api for MockApi {
    fn addr_validate(&self, input: &str) -> StdResult<Addr> {
        let canonical = self.addr_canonicalize(input)?;
        let normalized = self.addr_humanize(&canonical)?;
        if input != normalized.as_str() {
            return Err(StdError::generic_err(
                "Invalid input: address not normalized",
            ));
        }
        Ok(Addr::unchecked(input))
    }

    fn addr_canonicalize(&self, input: &str) -> StdResult<CanonicalAddr> {
        let hrp_str = CheckedHrpstring::new::<Bech32>(input)
            .map_err(|_| StdError::generic_err("Error decoding bech32"))?;

        if !hrp_str
            .hrp()
            .as_bytes()
            .eq_ignore_ascii_case(self.bech32_prefix.as_bytes())
        {
            return Err(StdError::generic_err("Wrong bech32 prefix"));
        }

        let bytes: Vec<u8> = hrp_str.byte_iter().collect();
        validate_length(&bytes)?;
        Ok(bytes.into())
    }

    fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
        validate_length(canonical.as_ref())?;

        let prefix = Hrp::parse(&self.bech32_prefix)
            .map_err(|_| StdError::generic_err("Invalid bech32 prefix"))?;
        encode::<Bech32>(prefix, canonical.as_slice())
            .map(Addr::unchecked)
            .map_err(|_| StdError::generic_err("Bech32 encoding error"))
    }

    fn bls12_381_aggregate_g1(&self, g1s: &[u8]) -> Result<[u8; 48], VerificationError> {
        cosmwasm_crypto::bls12_381_aggregate_g1(g1s).map_err(Into::into)
    }

    fn bls12_381_aggregate_g2(&self, g2s: &[u8]) -> Result<[u8; 96], VerificationError> {
        cosmwasm_crypto::bls12_381_aggregate_g2(g2s).map_err(Into::into)
    }

    fn bls12_381_pairing_equality(
        &self,
        ps: &[u8],
        qs: &[u8],
        r: &[u8],
        s: &[u8],
    ) -> Result<bool, VerificationError> {
        cosmwasm_crypto::bls12_381_pairing_equality(ps, qs, r, s).map_err(Into::into)
    }

    fn bls12_381_hash_to_g1(
        &self,
        hash_function: HashFunction,
        msg: &[u8],
        dst: &[u8],
    ) -> Result<[u8; 48], VerificationError> {
        Ok(cosmwasm_crypto::bls12_381_hash_to_g1(
            hash_function.into(),
            msg,
            dst,
        ))
    }

    fn bls12_381_hash_to_g2(
        &self,
        hash_function: HashFunction,
        msg: &[u8],
        dst: &[u8],
    ) -> Result<[u8; 96], VerificationError> {
        Ok(cosmwasm_crypto::bls12_381_hash_to_g2(
            hash_function.into(),
            msg,
            dst,
        ))
    }

    fn secp256k1_verify(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        Ok(cosmwasm_crypto::secp256k1_verify(
            message_hash,
            signature,
            public_key,
        )?)
    }

    fn secp256k1_recover_pubkey(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        recovery_param: u8,
    ) -> Result<Vec<u8>, RecoverPubkeyError> {
        let pubkey =
            cosmwasm_crypto::secp256k1_recover_pubkey(message_hash, signature, recovery_param)?;
        Ok(pubkey.to_vec())
    }

    fn secp256r1_verify(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        Ok(cosmwasm_crypto::secp256r1_verify(
            message_hash,
            signature,
            public_key,
        )?)
    }

    fn secp256r1_recover_pubkey(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        recovery_param: u8,
    ) -> Result<Vec<u8>, RecoverPubkeyError> {
        let pubkey =
            cosmwasm_crypto::secp256r1_recover_pubkey(message_hash, signature, recovery_param)?;
        Ok(pubkey.to_vec())
    }

    fn ed25519_verify(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        Ok(cosmwasm_crypto::ed25519_verify(
            message, signature, public_key,
        )?)
    }

    fn ed25519_batch_verify(
        &self,
        messages: &[&[u8]],
        signatures: &[&[u8]],
        public_keys: &[&[u8]],
    ) -> Result<bool, VerificationError> {
        Ok(cosmwasm_crypto::ed25519_batch_verify(
            &mut OsRng,
            messages,
            signatures,
            public_keys,
        )?)
    }

    fn debug(&self, #[allow(unused)] message: &str) {
        println!("{message}");
    }
}

/// Does basic validation of the number of bytes in a canonical address
fn validate_length(bytes: &[u8]) -> StdResult<()> {
    match bytes.len() {
        1..=255 => Ok(()),
        _ => Err(StdError::generic_err("Invalid canonical address length")),
    }
}
