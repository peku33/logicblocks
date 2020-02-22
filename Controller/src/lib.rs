#![allow(dead_code)]
#![feature(async_closure)]
#![feature(option_expect_none)]
#![feature(raw_vec_internals)]
#![feature(trait_alias)]
#![feature(try_blocks)]
#![feature(drain_filter)]
#![feature(poll_map)]
#![feature(raw)]
#![warn(clippy::all)]
#![allow(clippy::new_without_default)]
#![allow(clippy::type_complexity)]

pub mod devices;
pub mod modules;
pub mod util;
pub mod web;
