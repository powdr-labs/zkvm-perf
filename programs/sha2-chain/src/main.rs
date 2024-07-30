// Code taken from JOLT's benchmark repository.

#![no_main]
#![cfg_attr(feature = "powdr", no_std)]

#[cfg(feature = "powdr")]
extern crate powdr_riscv_runtime;

use sha2::{Digest, Sha256};

#[cfg(feature = "powdr")]
use core::hint::black_box;

#[cfg(not(feature = "powdr"))]
use std::hint::black_box;

#[cfg(feature = "risc0")]
risc0_zkvm::guest::entry!(run);

#[cfg(feature = "sp1")]
sp1_zkvm::entrypoint!(run);

fn run() {
    let input = [5u8; 32];
    let num_iters: u32 = 2500;
    let mut hash = input;
    for _ in 0..num_iters {
        let mut hasher = Sha256::new();
        hasher.update(hash);
        let res = &hasher.finalize();
        hash = Into::<[u8; 32]>::into(*res);
    }
}

#[cfg(feature = "powdr")]
#[no_mangle]
pub fn main() {
    run();
}

#[cfg(not(feature = "powdr"))]
pub fn main() {
    run();
}
