//! Tests for Dict type.

mod common;

use rayforce::{Dict, I64, RayString, RayType};
use serial_test::serial;

#[test]
#[serial]
fn test_dict_from_pairs() {
    init_runtime!();
    let dict = Dict::from_pairs([
        ("key1", I64::new(100).ptr().clone()),
        ("key2", I64::new(200).ptr().clone()),
    ])
    .unwrap();

    assert_eq!(dict.len(), 2);
}

#[test]
#[serial]
fn test_dict_empty() {
    init_runtime!();
    let dict = Dict::from_pairs(std::iter::empty::<(&str, rayforce::RayObj)>()).unwrap();
    // Empty dict was created successfully
    assert!(!dict.ptr().is_nil());
}

#[test]
#[serial]
fn test_dict_single_pair() {
    init_runtime!();
    let dict = Dict::from_pairs([("name", I64::new(42).ptr().clone())]).unwrap();
    // Verify dict was created
    assert!(!dict.ptr().is_nil());
}

#[test]
#[serial]
fn test_dict_get() {
    init_runtime!();
    let dict = Dict::from_pairs([("value", I64::new(123).ptr().clone())]).unwrap();

    let val = dict.get("value");
    assert!(val.is_some());
}

#[test]
#[serial]
fn test_dict_keys() {
    init_runtime!();
    let dict = Dict::from_pairs([
        ("a", I64::new(1).ptr().clone()),
        ("b", I64::new(2).ptr().clone()),
    ])
    .unwrap();

    let keys = dict.keys();
    // Keys should be a vector of symbols
    assert!(!keys.is_nil());
}

#[test]
#[serial]
fn test_dict_values() {
    init_runtime!();
    let dict = Dict::from_pairs([
        ("x", I64::new(10).ptr().clone()),
        ("y", I64::new(20).ptr().clone()),
    ])
    .unwrap();

    let values = dict.values();
    assert!(!values.is_nil());
}

#[test]
#[serial]
fn test_dict_mixed_types() {
    init_runtime!();
    let dict = Dict::from_pairs([
        ("name", RayString::new("Alice").ptr().clone()),
        ("age", I64::new(30).ptr().clone()),
    ])
    .unwrap();

    assert_eq!(dict.len(), 2);
}

#[test]
#[serial]
fn test_dict_display() {
    init_runtime!();
    let dict = Dict::from_pairs([("key", I64::new(1).ptr().clone())]).unwrap();

    let display = format!("{}", dict);
    assert!(!display.is_empty());
}

#[test]
#[serial]
fn test_dict_debug() {
    init_runtime!();
    let dict = Dict::from_pairs([("key", I64::new(1).ptr().clone())]).unwrap();

    let debug = format!("{:?}", dict);
    assert!(debug.contains("Dict"));
}

#[test]
#[serial]
fn test_dict_clone() {
    init_runtime!();
    let dict1 = Dict::from_pairs([("test", I64::new(42).ptr().clone())]).unwrap();

    let dict2 = dict1.clone();
    // Both should be valid
    assert!(!dict1.ptr().is_nil());
    assert!(!dict2.ptr().is_nil());
}

#[test]
#[serial]
fn test_dict_type_code() {
    init_runtime!();
    let dict = Dict::from_pairs([("key", I64::new(1).ptr().clone())]).unwrap();

    // Just verify we can get the type code
    let _code = dict.type_code();
}
