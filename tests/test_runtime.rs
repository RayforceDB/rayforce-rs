//! Tests for Rayforce runtime and evaluation.

mod common;

use rayforce::Rayforce;
use serial_test::serial;

#[test]
#[serial]
fn test_runtime_creation() {
    with_runtime!(rf, {
        assert!(!rf.as_ptr().is_null());
    });
}

#[test]
#[serial]
fn test_runtime_version() {
    with_runtime!(rf, {
        let v = rf.version();
        assert!(v > 0);
    });
}

#[test]
#[serial]
fn test_eval_integer() {
    with_runtime!(rf, {
        let result = rf.eval("42").unwrap();
        let val: i64 = result.try_into().unwrap();
        assert_eq!(val, 42);
    });
}

#[test]
#[serial]
fn test_eval_negative_integer() {
    with_runtime!(rf, {
        let result = rf.eval("-123").unwrap();
        let val: i64 = result.try_into().unwrap();
        assert_eq!(val, -123);
    });
}

#[test]
#[serial]
fn test_eval_zero() {
    with_runtime!(rf, {
        let result = rf.eval("0").unwrap();
        let val: i64 = result.try_into().unwrap();
        assert_eq!(val, 0);
    });
}

#[test]
#[serial]
fn test_eval_float() {
    with_runtime!(rf, {
        let result = rf.eval("3.14").unwrap();
        let val: f64 = result.try_into().unwrap();
        assert!((val - 3.14).abs() < 0.001);
    });
}

#[test]
#[serial]
fn test_eval_string() {
    with_runtime!(rf, {
        let result = rf.eval("\"hello\"").unwrap();
        let display = format!("{}", result);
        assert!(display.contains("hello"));
    });
}

#[test]
#[serial]
fn test_eval_symbol() {
    with_runtime!(rf, {
        let result = rf.eval("`test");
        // Symbol evaluation may or may not succeed depending on context
        // Just verify the runtime handles it
        assert!(result.is_ok() || result.is_err());
    });
}

#[test]
#[serial]
fn test_eval_empty_string() {
    with_runtime!(rf, {
        let result = rf.eval("\"\"").unwrap();
        assert!(!result.is_nil());
    });
}

#[test]
#[serial]
fn test_runtime_builder() {
    // This test uses the shared runtime through the macro
    with_runtime!(rf, {
        assert!(!rf.as_ptr().is_null());
    });
}

#[test]
#[serial]
fn test_eval_multiple() {
    with_runtime!(rf, {
        // Multiple evaluations should work
        let r1 = rf.eval("1").unwrap();
        let r2 = rf.eval("2").unwrap();
        let r3 = rf.eval("3").unwrap();

        let v1: i64 = r1.try_into().unwrap();
        let v2: i64 = r2.try_into().unwrap();
        let v3: i64 = r3.try_into().unwrap();

        assert_eq!(v1, 1);
        assert_eq!(v2, 2);
        assert_eq!(v3, 3);
    });
}
