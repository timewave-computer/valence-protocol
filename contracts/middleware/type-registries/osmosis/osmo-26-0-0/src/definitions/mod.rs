use bank_balance::OsmosisBankBalance;
use gamm_pool::OsmosisXykPool;
use osmosis_std::types::{
    cosmos::bank::v1beta1::QueryBalanceResponse, osmosis::gamm::v1beta1::Pool,
};
use valence_middleware_utils::register_types;

pub mod bank_balance;
pub mod gamm_pool;

// TODO: embed the previously deployed version identifier there
// to ensure that types declared here implement a 1-1 mapper from
// the outdated version to this one.

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
        native_type: Pool,
        adapter: OsmosisXykPool,
        to_valence: ValenceType::XykPool,
        // migrate_from: osmo_25_0_0::Pool,
    },
    "/cosmos.bank.v1beta1.QueryBalanceResponse" => {
        native_type: QueryBalanceResponse,
        adapter: OsmosisBankBalance,
        to_valence: ValenceType::BankBalance,
        // migrate_from: osmo_25_0_0::QueryBalanceResponse,
    }
}
