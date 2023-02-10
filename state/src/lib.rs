#![no_std]

mod state;

#[cfg(feature = "dummy")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));
