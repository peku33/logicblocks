#![feature(adt_const_params)]
#![feature(array_chunks)]
#![feature(array_windows)]
#![feature(associated_type_defaults)]
#![feature(async_fn_traits)]
#![feature(exact_size_is_empty)]
#![feature(inherent_associated_types)]
#![feature(int_roundings)]
#![feature(let_chains)]
#![feature(never_type)]
#![feature(sync_unsafe_cell)]
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
