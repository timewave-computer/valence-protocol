// The proto message that is going to be executed on Noble
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgDepositForBurn {
    /// the signer address
    #[prost(string, tag = "1")]
    pub from: ::prost::alloc::string::String,
    /// the amount to bridge
    #[prost(string, tag = "2")]
    pub amount: ::prost::alloc::string::String,
    /// the destination domain
    #[prost(uint32, tag = "3")]
    pub destination_domain: u32,
    /// the mint recipient address
    #[prost(bytes, tag = "4")]
    pub mint_recipient: ::prost::alloc::vec::Vec<u8>,
    /// the token denom that is being bridged
    #[prost(string, tag = "5")]
    pub burn_token: ::prost::alloc::string::String,
}

impl ::prost::Name for MsgDepositForBurn {
    const NAME: &'static str = "MsgDepositForBurn";
    const PACKAGE: &'static str = "circle.cctp.v1";
    fn full_name() -> ::prost::alloc::string::String {
        "circle.cctp.v1.MsgDepositForBurn".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/circle.cctp.v1.MsgDepositForBurn".into()
    }
}
