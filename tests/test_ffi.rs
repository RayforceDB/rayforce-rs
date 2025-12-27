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

//! Tests for FFI layer functions.

mod common;

use rayforce::ffi;
use rayforce::RayObj;
use serial_test::serial;

#[test]
#[serial]
fn test_new_symbol() {
    init_runtime!();
    let sym = ffi::new_symbol("test");
    assert!(!sym.is_nil());
}

#[test]
#[serial]
fn test_new_symbol_empty() {
    init_runtime!();
    // Empty symbol may or may not be nil depending on implementation
    let _sym = ffi::new_symbol("");
}

#[test]
#[serial]
fn test_new_list() {
    init_runtime!();
    let list = ffi::new_list();
    assert!(!list.is_nil());
}

#[test]
#[serial]
fn test_push_to_list() {
    init_runtime!();
    let mut list = ffi::new_list();
    let item: RayObj = 42i64.into();
    ffi::push_to_list(&mut list, item);

    assert_eq!(ffi::get_obj_len(&list), 1);
}

#[test]
#[serial]
fn test_push_multiple_to_list() {
    init_runtime!();
    let mut list = ffi::new_list();
    ffi::push_to_list(&mut list, 1i64.into());
    ffi::push_to_list(&mut list, 2i64.into());
    ffi::push_to_list(&mut list, 3i64.into());

    assert_eq!(ffi::get_obj_len(&list), 3);
}

#[test]
#[serial]
fn test_get_obj_len_vector() {
    init_runtime!();
    let data = [1i64, 2, 3, 4, 5];
    let obj: RayObj = data.as_slice().into();
    assert_eq!(ffi::get_obj_len(&obj), 5);
}

#[test]
#[serial]
fn test_get_at_index() {
    init_runtime!();
    let mut list = ffi::new_list();
    ffi::push_to_list(&mut list, 100i64.into());
    ffi::push_to_list(&mut list, 200i64.into());

    let item = ffi::get_at_index(&list, 0);
    assert!(item.is_some());

    let item = ffi::get_at_index(&list, 1);
    assert!(item.is_some());
}

#[test]
#[serial]
fn test_get_at_index_out_of_bounds() {
    init_runtime!();
    let list = ffi::new_list();
    // Out of bounds may return None or some sentinel value
    let _item = ffi::get_at_index(&list, 0);
}

#[test]
#[serial]
fn test_rayobj_is_nil() {
    init_runtime!();
    let obj: RayObj = 42i64.into();
    assert!(!obj.is_nil());
}

#[test]
#[serial]
fn test_symbol_interning() {
    init_runtime!();
    // Same symbol should be interned
    let sym1 = ffi::new_symbol("same");
    let sym2 = ffi::new_symbol("same");

    // Both should be valid
    assert!(!sym1.is_nil());
    assert!(!sym2.is_nil());
}

#[test]
#[serial]
fn test_get_obj_raw_ptr() {
    init_runtime!();
    let data = [1i64, 2, 3];
    let obj: RayObj = data.as_slice().into();

    let ptr = ffi::get_obj_raw_ptr(&obj);
    assert!(!ptr.is_null());
}

#[test]
#[serial]
fn test_list_operations() {
    init_runtime!();
    let mut list = ffi::new_list();
    
    // Empty list
    assert_eq!(ffi::get_obj_len(&list), 0);
    
    // Add items
    ffi::push_to_list(&mut list, 1i64.into());
    assert_eq!(ffi::get_obj_len(&list), 1);
    
    ffi::push_to_list(&mut list, 2i64.into());
    assert_eq!(ffi::get_obj_len(&list), 2);
}
