use strum::EnumString;

#[derive(Debug, EnumString, Default)]
#[strum(serialize_all = "snake_case")]
pub enum ForwarderFunction {
    #[default]
    Forward,
}
