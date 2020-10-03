#![feature(associated_type_defaults)]
#![feature(async_closure)]
#![feature(drain_filter)]
#![feature(exact_size_is_empty)]
#![feature(generic_associated_types)]
#![feature(never_type)]
#![feature(option_expect_none)]
#![feature(option_result_contains)]
#![feature(option_unwrap_none)]
#![feature(poll_map)]
#![feature(raw_vec_internals)]
#![feature(raw)]
#![feature(trait_alias)]
#![feature(try_blocks)]
#![warn(clippy::all)]
#![allow(clippy::new_without_default)]
#![allow(clippy::type_complexity)]
#![allow(dead_code)]
#![allow(incomplete_features)]
#![recursion_limit = "256"]

pub mod datatypes;
pub mod devices;
pub mod modules;
pub mod signals;
pub mod stubs;
pub mod util;
pub mod web;
