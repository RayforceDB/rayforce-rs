//! Phase 3: vectors (zero-copy), lists, dicts.

use rayforce::{eval, Runtime, Value};

#[test]
fn vector_zero_copy_slice() {
    let _rt = Runtime::new().unwrap();
    let data = [1i64, 2, 3, 4, 5];
    let v = Value::vec(&data);
    assert_eq!(v.len(), 5);
    assert_eq!(v.as_slice::<i64>().unwrap(), &data);
    // wrong element type is rejected
    assert!(v.as_slice::<i32>().is_err());
}

#[test]
fn vector_all_numeric_types() {
    let _rt = Runtime::new().unwrap();
    assert_eq!(
        Value::vec(&[1u8, 2, 3]).as_slice::<u8>().unwrap(),
        &[1, 2, 3]
    );
    assert_eq!(
        Value::vec(&[-1i16, 0, 1]).as_slice::<i16>().unwrap(),
        &[-1, 0, 1]
    );
    assert_eq!(
        Value::vec(&[10i32, 20]).as_slice::<i32>().unwrap(),
        &[10, 20]
    );
    assert_eq!(
        Value::vec(&[1.5f32, 2.5]).as_slice::<f32>().unwrap(),
        &[1.5, 2.5]
    );
    assert_eq!(
        Value::vec(&[1.0f64, 2.0]).as_slice::<f64>().unwrap(),
        &[1.0, 2.0]
    );
    assert_eq!(Value::bool_vec(&[true, false, true]).len(), 3);
}

#[test]
fn vector_get_and_iter() {
    let _rt = Runtime::new().unwrap();
    let v = Value::vec(&[10i64, 20, 30]);
    assert_eq!(v.get(0).unwrap().as_i64().unwrap(), 10);
    assert_eq!(v.get(2).unwrap().as_i64().unwrap(), 30);
    assert!(v.get(3).is_err());
    let collected: Vec<i64> = v.to_vec().unwrap();
    assert_eq!(collected, vec![10, 20, 30]);
    let via_iter: Vec<i64> = v.iter().map(|r| r.unwrap().as_i64().unwrap()).collect();
    assert_eq!(via_iter, vec![10, 20, 30]);
}

#[test]
fn vector_mutation() {
    let _rt = Runtime::new().unwrap();
    let mut v = Value::vec(&[1i64, 2, 3]);
    v.set(1, 99i64).unwrap();
    assert_eq!(v.as_slice::<i64>().unwrap(), &[1, 99, 3]);
    v.push(4i64).unwrap();
    assert_eq!(v.as_slice::<i64>().unwrap(), &[1, 99, 3, 4]);
    // type mismatch rejected
    assert!(v.set(0, 1.0f64).is_err());
}

#[test]
fn vector_slice_and_concat() {
    let _rt = Runtime::new().unwrap();
    let v = Value::vec(&[1i64, 2, 3, 4, 5]);
    let s = v.slice(1, 3).unwrap();
    assert_eq!(s.as_slice::<i64>().unwrap(), &[2, 3, 4]);
    let a = Value::vec(&[1i64, 2]);
    let b = Value::vec(&[3i64, 4]);
    let c = a.concat(&b).unwrap();
    assert_eq!(c.as_slice::<i64>().unwrap(), &[1, 2, 3, 4]);
}

#[test]
fn vector_nulls() {
    let _rt = Runtime::new().unwrap();
    // A raw buffer carrying a coincidental sentinel is NOT null until the
    // HAS_NULLS attribute is set — matching the engine's design.
    let raw = Value::vec(&[1i64, i64::MIN, 3]);
    assert!(!raw.is_null_at(1));

    // Explicitly marking an element null is the supported path.
    let mut v = Value::vec(&[1i64, 2, 3]);
    v.set_null(1, true).unwrap();
    assert!(v.is_null_at(1));
    assert!(v.get(1).unwrap().is_null());
    assert_eq!(v.get(0).unwrap().as_i64().unwrap(), 1);
}

#[test]
fn symbol_and_string_vectors() {
    let _rt = Runtime::new().unwrap();
    let syms = Value::sym_vec(&["aaa", "bbb", "ccc"]);
    assert_eq!(syms.len(), 3);
    assert_eq!(syms.get(1).unwrap().as_sym().unwrap(), "bbb");

    let strs = Value::str_vec(&["hello", "a longer value here", ""]);
    assert_eq!(strs.len(), 3);
    assert_eq!(strs.get(0).unwrap().as_string().unwrap(), "hello");
    assert_eq!(
        strs.get(1).unwrap().as_string().unwrap(),
        "a longer value here"
    );
}

#[test]
fn list_heterogeneous() {
    let _rt = Runtime::new().unwrap();
    let items = [Value::i64(42), Value::sym("x"), Value::f64(3.5)];
    let l = Value::list(&items);
    assert!(l.is_list());
    assert_eq!(l.len(), 3);
    assert_eq!(l.get(0).unwrap().as_i64().unwrap(), 42);
    assert_eq!(l.get(1).unwrap().as_sym().unwrap(), "x");
    assert_eq!(l.get(2).unwrap().as_f64().unwrap(), 3.5);

    // source values keep their own reference (list retained them)
    assert_eq!(items[0].as_i64().unwrap(), 42);
}

#[test]
fn list_push() {
    let _rt = Runtime::new().unwrap();
    let mut l = Value::empty_list(2);
    l.list_push(&Value::i64(1)).unwrap();
    l.list_push(&Value::sym("two")).unwrap();
    assert_eq!(l.len(), 2);
    assert_eq!(l.get(1).unwrap().as_sym().unwrap(), "two");
}

#[test]
fn dict_construction_and_lookup() {
    let _rt = Runtime::new().unwrap();
    let keys = Value::sym_vec(&["a", "b", "c"]);
    let vals = Value::vec(&[1i64, 2, 3]);
    let d = Value::dict(keys, vals);
    assert!(d.is_dict());
    assert_eq!(d.dict_len().unwrap(), 3);
    assert_eq!(
        d.dict_keys().unwrap().get(0).unwrap().as_sym().unwrap(),
        "a"
    );
    assert_eq!(
        d.dict_values().unwrap().as_slice::<i64>().unwrap(),
        &[1, 2, 3]
    );

    let got = d.dict_get(&Value::sym("b")).unwrap();
    assert_eq!(got.unwrap().as_i64().unwrap(), 2);
    assert!(d.dict_get(&Value::sym("missing")).unwrap().is_none());
}

#[test]
fn bool_and_temporal_slices() {
    let _rt = Runtime::new().unwrap();
    let b = Value::bool_vec(&[true, false, true]);
    assert_eq!(b.bool_slice().unwrap(), &[1u8, 0, 1]);

    let dates = Value::empty_vec(rayforce::sys::RAY_DATE as i8, 0);
    let _ = dates; // construct-by-slice for temporals comes via Value::vec on i32 raw later
                   // date/time/timestamp readers reject a plain i64 vector
    let v = Value::vec(&[1i64, 2, 3]);
    assert!(v.date_days_slice().is_err());
    assert!(v.timestamp_nanos_slice().is_err());
}

#[test]
fn sym_get_is_lossless() {
    let _rt = Runtime::new().unwrap();
    // get() boxes a symbol directly from its id (no string round-trip).
    let syms = Value::sym_vec(&["alpha", "beta"]);
    assert_eq!(syms.get(0).unwrap().as_sym().unwrap(), "alpha");
    assert_eq!(syms.get(1).unwrap().as_sym().unwrap(), "beta");
}

#[test]
fn set_out_of_range_errors_no_leak() {
    let _rt = Runtime::new().unwrap();
    let mut v = Value::vec(&[1i64, 2, 3]);
    assert!(v.set(5, 9i64).is_err());
    // vector is still usable and unchanged
    assert_eq!(v.as_slice::<i64>().unwrap(), &[1, 2, 3]);
}

#[test]
fn vector_matches_engine() {
    let _rt = Runtime::new().unwrap();
    // A constructed i64 vector formats like the engine's `(til 5)` (0 1 2 3 4).
    let v = Value::vec(&[0i64, 1, 2, 3, 4]);
    assert_eq!(v.format(), eval("(til 5)").unwrap().format());
}
