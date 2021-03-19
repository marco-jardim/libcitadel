// Citadel C bindings library (libcitadel)
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.

#![recursion_limit = "256"]
#![feature(try_trait)]
#![feature(vec_into_raw_parts)]
// Coding conventions
#![deny(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    unused_mut,
    unused_imports,
    //dead_code,
    //missing_docs
)]

#[macro_use]
extern crate amplify;
#[macro_use]
extern crate amplify_derive;
#[macro_use]
extern crate serde_with;

pub mod capi;
mod client;
pub mod error;
mod helpers;

pub use client::citadel_client_t;
pub use helpers::{TryAsStr, TryFromRaw, TryIntoRaw, TryIntoString};
