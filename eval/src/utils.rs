use core::time;
use std::{collections::VecDeque, env, fs, path::Path, time::Instant};

use sp1_reth_primitives::SP1RethInput;

use crate::{EvalArgs, ProgramId, ProverId};

#[allow(unused)]
pub fn get_elf(args: &EvalArgs) -> String {
    let mut program_dir = args.program.to_string();
    if args.program == ProgramId::Tendermint || args.program == ProgramId::Reth {
        program_dir += "-";
        program_dir += args.prover.to_string().as_str();
    }

    let current_dir = env::current_dir().expect("Failed to get current working directory");

    let mut elf_path = current_dir.join(format!(
        "programs/{}/target/riscv32im-succinct-zkvm-elf/release/{}",
        program_dir, program_dir
    ));

    if args.prover == ProverId::Risc0 {
        elf_path = current_dir.join(format!(
            "programs/{}/target/riscv32im-risc0-zkvm-elf/release/{}",
            program_dir, program_dir
        ));
    }

    let elf_path_str = elf_path.to_str().expect("Failed to convert path to string").to_string();
    println!("elf path: {}", elf_path_str);
    elf_path_str
}

fn read_brainfuck_and_convert(path: &Path) -> Vec<u32> {
    let content = fs::read_to_string(path).expect("error reading brainfuck program");
    let valid_chars = "><+-.,[]";
    content
        .chars()
        .filter(|c| valid_chars.contains(*c))
        .map(|b| b as u32)
        // interpreter stops at seeing a 0
        .chain(std::iter::once(0))
        .collect()
}

fn read_brainfuck_inputs(path: &Path) -> VecDeque<u32> {
    let content = fs::read_to_string(path).expect("error reading brainfuck input file");
    content
        .split(',')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .map(|x| x.parse::<u32>().unwrap())
        .collect()
}

pub fn get_brainfuck_input(args: &EvalArgs) -> (Vec<u32>, VecDeque<u32>) {
    match &args.program_inputs[..] {
        [program] => (read_brainfuck_and_convert(program.as_ref()), Default::default()),
        [program, input] => {
            (read_brainfuck_and_convert(program.as_ref()), read_brainfuck_inputs(input.as_ref()))
        }
        _ => panic!("Brainfuck interpreter requires a program and possibly inputs"),
    }
}

pub fn get_reth_input(args: &EvalArgs) -> SP1RethInput {
    let block_number = match &args.program_inputs[..] {
        [block_number] => block_number.parse::<u64>().expect("Invalid reth block number"),
        _ => panic!("Block number is required for Reth program"),
    };

    let current_dir = env::current_dir().expect("Failed to get current working directory");

    let blocks_dir = current_dir.join("eval").join("blocks");

    let file_path = blocks_dir.join(format!("{}.bin", block_number));

    if let Ok(bytes) = fs::read(file_path) {
        bincode::deserialize(&bytes).expect("Unable to deserialize input")
    } else {
        let blocks: Vec<String> = fs::read_dir(&blocks_dir)
            .unwrap_or_else(|_| panic!("Failed to read blocks directory: {:?}", blocks_dir))
            .filter_map(|entry| {
                entry
                    .ok()
                    .and_then(|e| e.path().file_stem().and_then(|n| n.to_str().map(String::from)))
            })
            .collect();

        panic!("Block {} not supported. Please choose from: {}", block_number, blocks.join(", "));
    }
}

pub fn time_operation<T, F: FnOnce() -> T>(operation: F) -> (T, time::Duration) {
    let start = Instant::now();
    let result = operation();
    let duration = start.elapsed();
    (result, duration)
}
