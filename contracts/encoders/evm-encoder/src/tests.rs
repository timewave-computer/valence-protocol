use crate::EVMLibraryFunction;

#[test]
fn test_valid_combinations() {
    assert!(EVMLibraryFunction::is_valid("forwarder", "forward"));
    assert!(!EVMLibraryFunction::is_valid("forwarder", "invalid"));
    assert!(!EVMLibraryFunction::is_valid("invalid", "forward"));
    // PascalCase variants should not work as strings
    assert!(!EVMLibraryFunction::is_valid("Forwarder", "Forward"));
}
