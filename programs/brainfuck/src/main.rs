// This code is borrowed from RISC Zero's benchmarks.
//
// Copyright 2024 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(any(feature = "sp1", feature = "risc0"), no_main)]
#[cfg(feature = "powdr")]
extern crate powdr_riscv_runtime;

#[cfg(feature = "risc0")]
risc0_zkvm::guest::entry!(main);

#[cfg(feature = "sp1")]
sp1_zkvm::entrypoint!(main);

mod interpreter;

use std::collections::VecDeque;

#[cfg(feature = "powdr")]
fn read_program_and_inputs() -> (Vec<u32>, VecDeque<i64>) {
    use powdr_riscv_runtime::io::read;
    (read(0), read(1))
}

#[cfg(feature = "sp1")]
fn read_program_and_inputs() -> (Vec<u32>, VecDeque<i64>) {
    sp1_zkvm::io::read()
}

#[cfg(feature = "risc0")]
fn read_program_and_inputs() -> (Vec<u32>, VecDeque<i64>) {
    risc0_zkvm::guest::env::read()
}

fn main() {
    let (program, inputs) = read_program_and_inputs();
    let (_, output) = interpreter::run(program, inputs);
    let output = String::from_utf8(output).unwrap();
    println!("{output}");
}
