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

//! Common test utilities and fixtures.
//!
//! Since only one Rayforce runtime can exist at a time,
//! tests must run serially (use #[serial] attribute).

use rayforce::{Rayforce, Result};

/// Create a new runtime for testing.
pub fn create_runtime() -> Result<Rayforce> {
    Rayforce::new()
}

/// Macro to run a test with a fresh runtime.
/// Usage: with_runtime!(rf, { ... })
#[macro_export]
macro_rules! with_runtime {
    ($rf:ident, $body:block) => {{
        let $rf = $crate::common::create_runtime().expect("Failed to create runtime");
        $body
    }};
}

/// Macro for tests that don't need the runtime reference but need it initialized.
/// Usage: init_runtime!();
#[macro_export]
macro_rules! init_runtime {
    () => {
        let _rf = $crate::common::create_runtime().expect("Failed to create runtime");
    };
}
