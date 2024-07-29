#![no_main]
#![cfg_attr(feature = "powdr", no_std)]

#[cfg(feature = "powdr")]
extern crate powdr_riscv_runtime;

#[cfg(feature = "risc0")]
risc0_zkvm::guest::entry!(main);

#[cfg(feature = "sp1")]
sp1_zkvm::entrypoint!(main);

fn fibonacci(n: u32) -> u32 {
    let mut a = 0;
    let mut b = 1;
    for _ in 0..n {
        let sum = (a + b) % 7919; // Mod to avoid overflow
        a = b;
        b = sum;
    }
    b
}

const N: u32 = 3;

#[cfg(feature = "powdr")]
#[no_mangle]
pub fn main() {
    use core::hint::black_box;
    use powdr_riscv_runtime::print;
    let result = black_box(fibonacci(black_box(N)));
    print!("result: {}", result);
}

#[cfg(not(feature = "powdr"))]
pub fn main() {
    use std::hint::black_box;
    let result = black_box(fibonacci(black_box(N)));
    println!("result: {}", result);
}
