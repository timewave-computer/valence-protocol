use cosmwasm_schema::{cw_serde, write_api};
use valence_program_manager::{
    program_config::ProgramConfig, program_migration::ProgramConfigMigrate,
    program_update::ProgramConfigUpdate,
};

#[cw_serde]
struct Types {
    program_config: ProgramConfig,
    program_config_update: ProgramConfigUpdate,
    program_config_migration: ProgramConfigMigrate,
}

fn main() {
    write_api! {
        instantiate: Types,
    }
}
