use bank_balance::OsmosisBankBalance;
use gamm_pool::OsmosisXykPool;
use osmosis_std::types::{
    cosmos::bank::v1beta1::QueryBalanceResponse, osmosis::gamm::v1beta1::Pool,
};
use valence_middleware_utils::register_types;

pub mod bank_balance;
pub mod gamm_pool;

register_types! {
    "/osmosis.gamm.v1beta1.Pool" => {
        // in the future, further plugins can be added here to handle type-specific
        // logic. e.g. a migration plugin that would handle the type conversion
        // from the type defined in the previous (semver) type registry:
        // migrate_from: osmo_25_0_0::Pool,
        // or maybe some kind of encoder/decoder plugin that could be defined along
        // the lines of:
        // evm_encoder: EvmTypeEncoder,
        native_type: Pool,
        adapter: OsmosisXykPool,
        to_valence: ValenceType::XykPool,
    },
    "/cosmos.bank.v1beta1.QueryBalanceResponse" => {
        native_type: QueryBalanceResponse,
        adapter: OsmosisBankBalance,
        to_valence: ValenceType::BankBalance,
    }
}
