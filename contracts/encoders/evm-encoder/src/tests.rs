use crate::EVMLibrary;

#[test]
fn test_valid_combinations() {
    assert!(EVMLibrary::is_valid("forwarder", "forward"));
    assert!(!EVMLibrary::is_valid("forwarder", "invalid"));
    assert!(!EVMLibrary::is_valid("invalid", "forward"));
    // PascalCase variants should not work as strings
    assert!(!EVMLibrary::is_valid("Forwarder", "Forward"));
}
