//! An implementation of a type-1, bytecompatible compatible, zkEVM written in Rust & SP1.
//!
//! The flow for the guest program is based on Zeth.
//!
//! Reference: https://github.com/risc0/zeth

#[cfg(feature = "powdr")]
extern crate powdr_riscv_runtime;

use reth_primitives::B256;
use revm::InMemoryDB;
use sp1_reth_primitives::{
    db::InMemoryDBHelper, mpt::keccak, processor::EvmProcessor, SP1RethInput,
};

// Include bytes from the file with the block number.

fn main() {
    // Read the input.
    let mut input = powdr_riscv_runtime::io::read::<SP1RethInput>(0);

    // Initialize the database.
    let db = InMemoryDB::initialize(&mut input).unwrap();

    // Execute the block.
    let mut executor = EvmProcessor::<InMemoryDB> { input, db: Some(db), header: None };
    executor.initialize();
    executor.execute();
    executor.finalize();

    // Print the resulting block hash.
    let hash = B256::from(keccak(alloy_rlp::encode(executor.header.unwrap())));
    println!("block hash: {}", hash);
}
