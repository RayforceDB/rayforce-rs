//! Shared between the q integration tests. Every file under `tests/` is its
//! own crate, so anything two of them need lives here and is pulled in with
//! `mod support;`. Each test crate uses a different subset, hence the allow.
#![allow(dead_code)]

pub mod wire;
