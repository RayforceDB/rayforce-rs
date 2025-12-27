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

//! Tests for List type.

mod common;

use rayforce::{F64, I64, List, RayString, RayType};
use serial_test::serial;

#[test]
#[serial]
fn test_list_new() {
    init_runtime!();
    let list = List::new();
    assert_eq!(list.len(), 0);
    assert!(list.is_empty());
}

#[test]
#[serial]
fn test_list_push_i64() {
    init_runtime!();
    let mut list = List::new();
    list.push(1i64);
    list.push(2i64);
    list.push(3i64);
    assert_eq!(list.len(), 3);
}

#[test]
#[serial]
fn test_list_push_mixed() {
    init_runtime!();
    let mut list = List::new();
    list.push(42i64);
    list.push(3.14f64);
    list.push("hello");
    assert_eq!(list.len(), 3);
}

#[test]
#[serial]
fn test_list_get() {
    init_runtime!();
    let mut list = List::new();
    list.push(100i64);
    list.push(200i64);

    let item = list.get(0);
    assert!(item.is_some());

    let item = list.get(1);
    assert!(item.is_some());

    let item = list.get(2);
    assert!(item.is_none());
}

#[test]
#[serial]
fn test_list_iter() {
    init_runtime!();
    let mut list = List::new();
    list.push(1i64);
    list.push(2i64);
    list.push(3i64);

    let items: Vec<_> = list.iter().collect();
    assert_eq!(items.len(), 3);
}

#[test]
#[serial]
fn test_list_from_iter() {
    init_runtime!();
    let list = List::from_iter([1i64, 2i64, 3i64]);
    assert_eq!(list.len(), 3);
}

#[test]
#[serial]
fn test_list_with_typed_values() {
    init_runtime!();
    let mut list = List::new();
    list.push(I64::new(42).ptr().clone());
    list.push(F64::new(3.14).ptr().clone());
    list.push(RayString::new("test").ptr().clone());
    assert_eq!(list.len(), 3);
}

#[test]
#[serial]
fn test_list_large() {
    init_runtime!();
    let mut list = List::new();
    for i in 0..100 {
        list.push(i as i64);
    }
    assert_eq!(list.len(), 100);
}

#[test]
#[serial]
fn test_list_display() {
    init_runtime!();
    let mut list = List::new();
    list.push(1i64);
    list.push(2i64);
    let display = format!("{}", list);
    // Should display the list contents
    assert!(!display.is_empty());
}

#[test]
#[serial]
fn test_list_debug() {
    init_runtime!();
    let mut list = List::new();
    list.push(1i64);
    let debug = format!("{:?}", list);
    assert!(debug.contains("List"));
}

#[test]
#[serial]
fn test_list_default() {
    init_runtime!();
    let list: List = Default::default();
    assert!(list.is_empty());
}

#[test]
#[serial]
fn test_list_collect() {
    init_runtime!();
    let list: List = [1i64, 2, 3].into_iter().collect();
    assert_eq!(list.len(), 3);
}
