#![no_std]

mod contract;

#[cfg(feature = "dummy")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));
