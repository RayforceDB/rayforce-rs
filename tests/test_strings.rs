/*
*   Copyright (c) 2025 Anton Kundenko <singaraiona@gmail.com>
*   All rights reserved.

*   Permission is hereby granted, free of charge, to any person obtaining a copy
*   of this software and associated documentation files (the "Software"), to deal
*   in the Software without restriction, including without limitation the rights
*   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
*   copies of the Software, and to permit persons to whom the Software is
*   furnished to do so, subject to the following conditions:

*   The above copyright notice and this permission notice shall be included in all
*   copies or substantial portions of the Software.

*   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
*   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
*   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
*   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
*   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
*   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
*   SOFTWARE.
*/

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
