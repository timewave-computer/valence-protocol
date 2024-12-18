use crate::EVMLibraryFunction;

#[test]
fn test_valid_combinations() {
    assert!(EVMLibraryFunction::is_valid("forwarder"));
    assert!(!EVMLibraryFunction::is_valid("invalid"));
    // PascalCase variants should not work as strings
    assert!(!EVMLibraryFunction::is_valid("Forwarder"));
}
