//! Linkage + liveness smoke test for the raw FFI.
//!
//! Proves the static `librayforce.a` links and the core actually runs: spin up
//! a runtime, evaluate `1+1`, and read back the formatted result.

use std::ffi::{CStr, CString};

#[test]
fn eval_one_plus_one() {
    unsafe {
        let rt = rayforce_sys::ray_runtime_create(0, std::ptr::null_mut());
        assert!(!rt.is_null(), "runtime creation failed");

        let src = CString::new("(+ 1 1)").unwrap();
        let result = rayforce_sys::ray_eval_str(src.as_ptr());
        assert!(!result.is_null(), "eval returned null");

        // Format the result to a STR atom and read its bytes.
        let formatted = rayforce_sys::ray_fmt(result, 1);
        assert!(!formatted.is_null(), "ray_fmt returned null");

        let ptr = rayforce_sys::ray_str_ptr(formatted);
        assert!(!ptr.is_null(), "ray_str_ptr returned null");
        let text = CStr::from_ptr(ptr).to_string_lossy().into_owned();
        assert!(
            text.contains('2'),
            "expected formatted result to contain '2', got {text:?}"
        );

        rayforce_sys::ray_runtime_destroy(rt);
    }
}
