#![feature(array_windows)]
#![feature(exact_size_is_empty)]
#![feature(never_type)]
#![feature(trait_alias)]
#![feature(try_blocks)]
#![allow(dead_code)]
#![allow(incomplete_features)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::get_first)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::let_and_return)]
#![allow(clippy::mem_replace_with_default)]
#![allow(clippy::new_without_default)]
#![allow(clippy::nonminimal_bool)]
#![recursion_limit = "256"]

pub mod app;
pub mod datatypes;
pub mod devices;
pub mod gui;
pub mod interfaces;
pub mod modules;
pub mod signals;
pub mod stubs;
pub mod util;
pub mod web;
