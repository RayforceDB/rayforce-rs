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

//! Tests for Vector types.

mod common;

use rayforce::{Symbol, Vector};
use serial_test::serial;

#[test]
#[serial]
fn test_i64_vector_creation() {
    init_runtime!();
    let vec = Vector::<i64>::from_iter([1i64, 2, 3, 4, 5]);
    assert_eq!(vec.len(), 5);
}

#[test]
#[serial]
fn test_i64_vector_as_slice() {
    init_runtime!();
    let vec = Vector::<i64>::from_iter([10i64, 20, 30]);
    let slice = vec.as_slice();
    assert_eq!(slice, &[10, 20, 30]);
}

#[test]
#[serial]
fn test_i64_vector_get() {
    init_runtime!();
    let vec = Vector::<i64>::from_iter([100i64, 200, 300]);
    assert_eq!(vec.get(0), Some(100));
    assert_eq!(vec.get(1), Some(200));
    assert_eq!(vec.get(2), Some(300));
    assert_eq!(vec.get(3), None);
}

#[test]
#[serial]
fn test_i64_vector_set() {
    init_runtime!();
    let mut vec = Vector::<i64>::from_iter([1i64, 2, 3]);
    vec.set(1, 999);
    assert_eq!(vec.get(1), Some(999));
}

#[test]
#[serial]
fn test_i64_vector_empty() {
    init_runtime!();
    let vec = Vector::<i64>::from_iter(std::iter::empty::<i64>());
    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());
}

#[test]
#[serial]
fn test_i64_vector_single_element() {
    init_runtime!();
    let vec = Vector::<i64>::from_iter([42i64]);
    assert_eq!(vec.len(), 1);
    assert_eq!(vec.get(0), Some(42));
}

#[test]
#[serial]
fn test_i64_vector_large() {
    init_runtime!();
    let data: Vec<i64> = (0..1000).collect();
    let vec = Vector::<i64>::from_iter(data.clone());
    assert_eq!(vec.len(), 1000);
    assert_eq!(vec.get(0), Some(0));
    assert_eq!(vec.get(999), Some(999));
}

#[test]
#[serial]
fn test_f64_vector_creation() {
    init_runtime!();
    let vec = Vector::<f64>::from_iter([1.1, 2.2, 3.3]);
    assert_eq!(vec.len(), 3);
}

#[test]
#[serial]
fn test_f64_vector_as_slice() {
    init_runtime!();
    let vec = Vector::<f64>::from_iter([1.5, 2.5, 3.5]);
    let slice = vec.as_slice();
    assert_eq!(slice.len(), 3);
    assert!((slice[0] - 1.5).abs() < 1e-10);
    assert!((slice[1] - 2.5).abs() < 1e-10);
    assert!((slice[2] - 3.5).abs() < 1e-10);
}

#[test]
#[serial]
fn test_f64_vector_get() {
    init_runtime!();
    let vec = Vector::<f64>::from_iter([3.14, 2.71, 1.41]);
    assert!((vec.get(0).unwrap() - 3.14).abs() < 1e-10);
    assert!((vec.get(1).unwrap() - 2.71).abs() < 1e-10);
    assert!((vec.get(2).unwrap() - 1.41).abs() < 1e-10);
}

#[test]
#[serial]
fn test_f64_vector_set() {
    init_runtime!();
    let mut vec = Vector::<f64>::from_iter([0.0, 0.0, 0.0]);
    vec.set(0, 1.1);
    vec.set(1, 2.2);
    vec.set(2, 3.3);
    assert!((vec.get(0).unwrap() - 1.1).abs() < 1e-10);
    assert!((vec.get(1).unwrap() - 2.2).abs() < 1e-10);
    assert!((vec.get(2).unwrap() - 3.3).abs() < 1e-10);
}

#[test]
#[serial]
fn test_f64_vector_negative() {
    init_runtime!();
    let vec = Vector::<f64>::from_iter([-1.0, -2.0, -3.0]);
    let slice = vec.as_slice();
    assert!((slice[0] - (-1.0)).abs() < 1e-10);
    assert!((slice[1] - (-2.0)).abs() < 1e-10);
    assert!((slice[2] - (-3.0)).abs() < 1e-10);
}

#[test]
#[serial]
fn test_symbol_vector_creation() {
    init_runtime!();
    let vec = Vector::<Symbol>::from_iter(["apple", "banana", "cherry"]);
    assert_eq!(vec.len(), 3);
}

#[test]
#[serial]
fn test_symbol_vector_get() {
    init_runtime!();
    let vec = Vector::<Symbol>::from_iter(["hello", "world"]);
    assert_eq!(vec.get(0), Some("hello".to_string()));
    assert_eq!(vec.get(1), Some("world".to_string()));
    assert_eq!(vec.get(2), None);
}

#[test]
#[serial]
fn test_symbol_vector_empty() {
    init_runtime!();
    let vec = Vector::<Symbol>::from_iter(std::iter::empty::<&str>());
    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());
}

#[test]
#[serial]
fn test_symbol_vector_single() {
    init_runtime!();
    let vec = Vector::<Symbol>::from_iter(["single"]);
    assert_eq!(vec.len(), 1);
    assert_eq!(vec.get(0), Some("single".to_string()));
}

#[test]
#[serial]
fn test_symbol_vector_with_numbers() {
    init_runtime!();
    let vec = Vector::<Symbol>::from_iter(["item1", "item2", "item3"]);
    assert_eq!(vec.get(0), Some("item1".to_string()));
    assert_eq!(vec.get(1), Some("item2".to_string()));
    assert_eq!(vec.get(2), Some("item3".to_string()));
}

#[test]
#[serial]
fn test_vector_clone() {
    init_runtime!();
    let vec1 = Vector::<i64>::from_iter([1i64, 2, 3]);
    let vec2 = vec1.clone();
    assert_eq!(vec1.len(), vec2.len());
    assert_eq!(vec1.as_slice(), vec2.as_slice());
}

#[test]
#[serial]
fn test_vector_display() {
    init_runtime!();
    let vec = Vector::<i64>::from_iter([1i64, 2, 3]);
    let display = format!("{}", vec);
    // Display should contain the values
    assert!(display.contains("1") && display.contains("2") && display.contains("3"));
}

#[test]
#[serial]
fn test_vector_debug() {
    init_runtime!();
    let vec = Vector::<i64>::from_iter([1i64, 2, 3]);
    let debug = format!("{:?}", vec);
    assert!(debug.contains("Vector"));
    assert!(debug.contains("3")); // length
}
