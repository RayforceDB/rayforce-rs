/*
*   Copyright (c) 2025-2026 Anton Kundenko <singaraiona@gmail.com>
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

//! Minimal Rayfall REPL: read a line, evaluate it, print the result.
//!
//! The 1.0 version of this example used line editing, ANSI colour, and
//! several internal helpers (`get_internal_function`, `binary_set`,
//! ...) that don't exist in 2.0 yet. This trimmed cut keeps just the
//! read → eval → print loop.

use rayforce::{Rayforce, RayforceError, Result};
use std::io::{self, BufRead, Write};

fn main() -> Result<()> {
    let rf = Rayforce::new()?;
    println!("Rayforce {} REPL — type :q to quit.", rf.version());

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut line = String::new();

    loop {
        line.clear();
        write!(stdout, "‣ ").ok();
        stdout.flush().ok();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("read error: {e}");
                break;
            }
        }
        let src = line.trim();
        if src.is_empty() {
            continue;
        }
        if src == ":q" || src == ":quit" {
            break;
        }
        match rf.eval(src) {
            Ok(obj) => {
                if !obj.is_nil() {
                    println!("{obj}");
                }
            }
            Err(RayforceError::Ray { code, message, .. }) => {
                eprintln!("error[{code}]: {message}");
            }
            Err(e) => {
                eprintln!("error: {e}");
            }
        }
    }
    Ok(())
}
