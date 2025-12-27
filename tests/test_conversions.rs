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

//! Tests for type conversions.

mod common;

use rayforce::RayObj;
use serial_test::serial;

#[test]
#[serial]
fn test_i64_to_rayobj() {
    init_runtime!();
    let obj: RayObj = 42i64.into();
    assert!(!obj.is_nil());
    // Verify display shows the value
    assert!(format!("{}", obj).contains("42"));
}

#[test]
#[serial]
fn test_i64_negative_to_rayobj() {
    init_runtime!();
    let obj: RayObj = (-100i64).into();
    assert!(!obj.is_nil());
    // Just verify it's not nil - display format may vary
}

#[test]
#[serial]
fn test_i32_to_rayobj() {
    init_runtime!();
    let obj: RayObj = 42i32.into();
    assert!(!obj.is_nil());
}

#[test]
#[serial]
fn test_i16_to_rayobj() {
    init_runtime!();
    let obj: RayObj = 42i16.into();
    assert!(!obj.is_nil());
}

#[test]
#[serial]
fn test_f64_to_rayobj() {
    init_runtime!();
    let obj: RayObj = 3.14f64.into();
    assert!(!obj.is_nil());
    // Just verify it's not nil - display format may vary
}

#[test]
#[serial]
fn test_bool_true_to_rayobj() {
    init_runtime!();
    let obj: RayObj = true.into();
    assert!(!obj.is_nil());
}

#[test]
#[serial]
fn test_bool_false_to_rayobj() {
    init_runtime!();
    let obj: RayObj = false.into();
    assert!(!obj.is_nil());
}

#[test]
#[serial]
fn test_u8_to_rayobj() {
    init_runtime!();
    let obj: RayObj = 255u8.into();
    assert!(!obj.is_nil());
}

#[test]
#[serial]
fn test_str_to_rayobj() {
    init_runtime!();
    let obj: RayObj = "hello".into();
    assert!(!obj.is_nil());
}

#[test]
#[serial]
fn test_string_to_rayobj() {
    init_runtime!();
    let obj: RayObj = String::from("hello").into();
    assert!(!obj.is_nil());
}

#[test]
#[serial]
fn test_i64_slice_to_rayobj() {
    init_runtime!();
    let data = [1i64, 2, 3, 4, 5];
    let obj: RayObj = data.as_slice().into();
    assert!(!obj.is_nil());
    assert_eq!(obj.len(), 5);
}

#[test]
#[serial]
fn test_f64_slice_to_rayobj() {
    init_runtime!();
    let data = [1.0f64, 2.0, 3.0];
    let obj: RayObj = data.as_slice().into();
    assert!(!obj.is_nil());
    assert_eq!(obj.len(), 3);
}

#[test]
#[serial]
fn test_rayobj_clone() {
    init_runtime!();
    let obj1: RayObj = 42i64.into();
    let obj2 = obj1.clone();

    // Both should display the same value
    assert_eq!(format!("{}", obj1), format!("{}", obj2));
}

#[test]
#[serial]
fn test_rayobj_display() {
    init_runtime!();
    let obj: RayObj = 42i64.into();
    let display = format!("{}", obj);
    assert!(display.contains("42"));
}

#[test]
#[serial]
fn test_rayobj_debug() {
    init_runtime!();
    let obj: RayObj = 42i64.into();
    let debug = format!("{:?}", obj);
    assert!(!debug.is_empty());
}

#[test]
#[serial]
fn test_rayobj_type_code() {
    init_runtime!();
    let i64_obj: RayObj = 42i64.into();
    let f64_obj: RayObj = 3.14f64.into();
    let str_obj: RayObj = "hello".into();

    // Different types should have different type codes (magnitude)
    assert_ne!(
        i64_obj.type_code().abs(),
        f64_obj.type_code().abs()
    );
    assert_ne!(
        f64_obj.type_code().abs(),
        str_obj.type_code().abs()
    );
}

#[test]
#[serial]
fn test_rayobj_len_vector() {
    init_runtime!();
    let data = [1i64, 2, 3, 4, 5];
    let obj: RayObj = data.as_slice().into();
    assert_eq!(obj.len(), 5);
}

#[test]
#[serial]
fn test_rayobj_len_string() {
    init_runtime!();
    let obj: RayObj = "hello".into();
    assert_eq!(obj.len(), 5);
}

#[test]
#[serial]
fn test_empty_string_to_rayobj() {
    init_runtime!();
    let obj: RayObj = "".into();
    assert!(!obj.is_nil());
    assert_eq!(obj.len(), 0);
}

#[test]
#[serial]
fn test_empty_slice_to_rayobj() {
    init_runtime!();
    let data: [i64; 0] = [];
    let obj: RayObj = data.as_slice().into();
    assert!(!obj.is_nil());
    assert_eq!(obj.len(), 0);
}
