//! Tests for RayString type.

mod common;

use rayforce::{RayString, RayType};
use serial_test::serial;

#[test]
#[serial]
fn test_string_creation() {
    init_runtime!();
    let s = RayString::new("hello");
    assert_eq!(s.to_string(), "hello");
}

#[test]
#[serial]
fn test_string_empty() {
    init_runtime!();
    let s = RayString::new("");
    assert_eq!(s.to_string(), "");
    assert!(s.is_empty());
}

#[test]
#[serial]
fn test_string_len() {
    init_runtime!();
    let s = RayString::new("hello");
    assert_eq!(s.len(), 5);
}

#[test]
#[serial]
fn test_string_is_empty() {
    init_runtime!();
    let empty = RayString::new("");
    let non_empty = RayString::new("x");

    assert!(empty.is_empty());
    assert!(!non_empty.is_empty());
}

#[test]
#[serial]
fn test_string_unicode() {
    init_runtime!();
    let s = RayString::new("こんにちは");
    assert!(!s.is_empty());
}

#[test]
#[serial]
fn test_string_with_spaces() {
    init_runtime!();
    let s = RayString::new("hello world");
    assert_eq!(s.to_string(), "hello world");
}

#[test]
#[serial]
fn test_string_with_numbers() {
    init_runtime!();
    let s = RayString::new("test123");
    assert_eq!(s.to_string(), "test123");
}

#[test]
#[serial]
fn test_string_with_special_chars() {
    init_runtime!();
    let s = RayString::new("!@#$%");
    assert_eq!(s.to_string(), "!@#$%");
}

#[test]
#[serial]
fn test_string_long() {
    init_runtime!();
    let long_str = "a".repeat(1000);
    let s = RayString::new(&long_str);
    assert_eq!(s.len(), 1000);
    assert_eq!(s.to_string(), long_str);
}

#[test]
#[serial]
fn test_string_display() {
    init_runtime!();
    let s = RayString::new("test");
    assert_eq!(format!("{}", s), "test");
}

#[test]
#[serial]
fn test_string_debug() {
    init_runtime!();
    let s = RayString::new("test");
    let debug = format!("{:?}", s);
    assert!(debug.contains("String"));
    assert!(debug.contains("test"));
}

#[test]
#[serial]
fn test_string_from_str() {
    init_runtime!();
    let s: RayString = "hello".into();
    assert_eq!(s.to_string(), "hello");
}

#[test]
#[serial]
fn test_string_from_string() {
    init_runtime!();
    let s: RayString = String::from("hello").into();
    assert_eq!(s.to_string(), "hello");
}

#[test]
#[serial]
fn test_string_clone() {
    init_runtime!();
    let s1 = RayString::new("original");
    let s2 = s1.clone();
    assert_eq!(s1.to_string(), s2.to_string());
}

#[test]
#[serial]
fn test_string_type_code() {
    init_runtime!();
    let s = RayString::new("test");
    // String type should have the C8 type code
    assert_eq!(s.type_code(), RayString::TYPE_CODE);
}
