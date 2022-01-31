#![feature(array_chunks)]
#![feature(associated_type_defaults)]
#![feature(async_closure)]
#![feature(div_duration)]
#![feature(drain_filter)]
#![feature(exact_size_is_empty)]
#![feature(generic_associated_types)]
#![feature(hash_drain_filter)]
#![feature(inherent_associated_types)]
#![feature(int_roundings)]
#![feature(never_type)]
#![feature(option_result_contains)]
#![feature(raw_vec_internals)]
#![feature(trait_alias)]
#![feature(try_blocks)]
#![allow(dead_code)]
#![allow(incomplete_features)]
#![warn(clippy::all)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::from_over_into)] // https://github.com/rust-lang/rust-clippy/issues/6607
#![allow(clippy::let_and_return)]
#![allow(clippy::mem_replace_with_default)]
#![allow(clippy::new_without_default)]
#![allow(clippy::undropped_manually_drops)]
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
