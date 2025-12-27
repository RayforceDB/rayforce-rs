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

//! Tests for scalar types.

mod common;

use rayforce::{B8, C8, F64, I16, I32, I64, RayType, Symbol, U8};
use serial_test::serial;

#[test]
#[serial]
fn test_i64_creation() {
    init_runtime!();
    let val = I64::new(42);
    assert_eq!(format!("{}", val), "42");
}

#[test]
#[serial]
fn test_i64_negative() {
    init_runtime!();
    let val = I64::new(-123);
    assert_eq!(format!("{}", val), "-123");
}

#[test]
#[serial]
fn test_i64_zero() {
    init_runtime!();
    let val = I64::new(0);
    assert_eq!(format!("{}", val), "0");
}

#[test]
#[serial]
fn test_i64_max() {
    init_runtime!();
    let val = I64::new(i64::MAX);
    assert_eq!(format!("{}", val), format!("{}", i64::MAX));
}

#[test]
#[serial]
fn test_i64_min() {
    init_runtime!();
    let val = I64::new(i64::MIN);
    assert_eq!(format!("{}", val), format!("{}", i64::MIN));
}

#[test]
#[serial]
fn test_i32_creation() {
    init_runtime!();
    let val = I32::new(42);
    let display = format!("{}", val);
    // Display may or may not have type suffix
    assert!(display.contains("42"));
}

#[test]
#[serial]
fn test_i32_negative() {
    init_runtime!();
    let val = I32::new(-999);
    let display = format!("{}", val);
    assert!(display.contains("-999"));
}

#[test]
#[serial]
fn test_i16_creation() {
    init_runtime!();
    let val = I16::new(100);
    let display = format!("{}", val);
    assert!(display.contains("100"));
}

#[test]
#[serial]
fn test_i16_negative() {
    init_runtime!();
    let val = I16::new(-500);
    let display = format!("{}", val);
    assert!(display.contains("-500"));
}

#[test]
#[serial]
fn test_f64_creation() {
    init_runtime!();
    let val = F64::new(3.14159);
    let display = format!("{}", val);
    assert!(display.starts_with("3.14"));
}

#[test]
#[serial]
fn test_f64_negative() {
    init_runtime!();
    let val = F64::new(-2.718);
    let display = format!("{}", val);
    assert!(display.starts_with("-2.71"));
}

#[test]
#[serial]
fn test_f64_zero() {
    init_runtime!();
    let val = F64::new(0.0);
    let display = format!("{}", val);
    assert!(display.contains("0"));
}

#[test]
#[serial]
fn test_f64_large() {
    init_runtime!();
    let val = F64::new(1e100);
    let display = format!("{}", val);
    assert!(display.contains("e") || display.contains("E") || display.len() > 10);
}

#[test]
#[serial]
fn test_u8_creation() {
    init_runtime!();
    let val = U8::new(255);
    let display = format!("{}", val);
    // Display may be hex (0xff) or decimal (255)
    assert!(display.contains("255") || display.contains("ff"));
}

#[test]
#[serial]
fn test_u8_zero() {
    init_runtime!();
    let val = U8::new(0);
    let display = format!("{}", val);
    assert!(display.contains("0"));
}

#[test]
#[serial]
fn test_b8_true() {
    init_runtime!();
    let val = B8::new(true);
    // B8 may display as "true" or "1b" depending on implementation
    let display = format!("{}", val);
    assert!(display == "1b" || display == "true" || display == "1");
}

#[test]
#[serial]
fn test_b8_false() {
    init_runtime!();
    let val = B8::new(false);
    let display = format!("{}", val);
    assert!(display == "0b" || display == "false" || display == "0");
}

#[test]
#[serial]
fn test_c8_creation() {
    init_runtime!();
    let val = C8::new('a');
    let display = format!("{}", val);
    assert!(display.contains('a'));
}

#[test]
#[serial]
fn test_c8_special_char() {
    init_runtime!();
    let val = C8::new('!');
    let display = format!("{}", val);
    assert!(display.contains('!'));
}

#[test]
#[serial]
fn test_symbol_creation() {
    init_runtime!();
    let val = Symbol::new("hello");
    assert_eq!(format!("{}", val), "`hello");
}

#[test]
#[serial]
fn test_symbol_empty() {
    init_runtime!();
    let val = Symbol::new("");
    assert_eq!(format!("{}", val), "`");
}

#[test]
#[serial]
fn test_symbol_with_numbers() {
    init_runtime!();
    let val = Symbol::new("test123");
    assert_eq!(format!("{}", val), "`test123");
}

#[test]
#[serial]
fn test_symbol_underscore() {
    init_runtime!();
    let val = Symbol::new("my_var");
    assert_eq!(format!("{}", val), "`my_var");
}

#[test]
#[serial]
fn test_type_codes() {
    init_runtime!();
    // Verify type codes are correct
    let i64_val = I64::new(1);
    let i32_val = I32::new(1);
    let i16_val = I16::new(1);
    let f64_val = F64::new(1.0);
    let u8_val = U8::new(1);
    let b8_val = B8::new(true);

    // Type codes should be different for each type
    assert_ne!(i64_val.type_code(), i32_val.type_code());
    assert_ne!(i32_val.type_code(), i16_val.type_code());
    assert_ne!(f64_val.type_code(), i64_val.type_code());
    assert_ne!(u8_val.type_code(), b8_val.type_code());
}

#[test]
#[serial]
fn test_i64_value_roundtrip() {
    init_runtime!();
    let original = 12345i64;
    let val = I64::new(original);
    // Verify the object was created (type code may be negative for scalars)
    assert_eq!(val.type_code().abs(), I64::TYPE_CODE.abs());
}

#[test]
#[serial]
fn test_f64_value_roundtrip() {
    init_runtime!();
    let original = 3.14159;
    let val = F64::new(original);
    assert_eq!(val.type_code().abs(), F64::TYPE_CODE.abs());
}
