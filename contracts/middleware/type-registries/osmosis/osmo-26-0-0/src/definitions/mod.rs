use bank_balance::OsmosisBankBalance;
use gamm_pool::OsmosisXykPool;
use osmosis_std::types::{
    cosmos::bank::v1beta1::QueryBalanceResponse, osmosis::gamm::v1beta1::Pool,
};
use valence_middleware_utils::register_types;

pub mod bank_balance;
pub mod gamm_pool;

// thought: these definitions could also be treated as optional fields.
// e.g. not every type needs to be ICQ-able, so some types could miss
// the ICQ adapter implementation. If registry receives an ICQ request
// regarding a type that does not have an ICQ adapter, we return a clear
// error saying that the type is not ICQ-able (and perhaps provide the
// functionality that is available).
// such optionality could also enable us to make use of the semver more
// extensively. for instance, the major/minor/patch versions could follow
// the upstream type, and various additions could be attached to the semver
// as pre-release identifiers.

register_types! {
    "/osmosis.gamm.v1beta1.Pool" => {
        // in the future, further plugins can be added here to handle type-specific
        // logic. e.g. a migration plugin that would handle the type conversion
        // from the type defined in the previous (semver) type registry:
        // migrate_from: osmo_25_0_0::Pool,
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
