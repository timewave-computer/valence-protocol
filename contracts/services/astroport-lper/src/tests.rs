use valence_astroport_utils::suite::AstroportTestAppBuilder;

#[test]
pub fn test_builder() {
    AstroportTestAppBuilder::new().build().unwrap();
}
