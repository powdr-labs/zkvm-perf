use std::fs;

use crate::{utils::*, EvalArgs, PerformanceReport, ProgramId};

use sp1_prover::components::DefaultProverComponents;
use sp1_sdk::{utils, SP1Context, SP1Prover, SP1Stdin};

pub struct SP1Evaluator;

impl SP1Evaluator {
    pub fn eval(args: &EvalArgs) -> PerformanceReport {
        // Setup the logger.
        utils::setup_logger();

        // Set enviroment variables to configure the prover.
        std::env::set_var("SHARD_SIZE", format!("{}", 1 << args.shard_size));
        if args.program == ProgramId::Reth {
            std::env::set_var("SHARD_CHUNKING_MULTIPLIER", "4");
        }

        // set program inputs
        let stdin = match args.program {
            ProgramId::Brainfuck => {
                let input = get_brainfuck_input(args);
                let mut stdin = SP1Stdin::new();
                stdin.write(&input);
                stdin
            }
            ProgramId::Reth => {
                let input = get_reth_input(args);
                let mut stdin = SP1Stdin::new();
                stdin.write(&input);
                stdin
            }
            ProgramId::BrainfuckAsm | ProgramId::BrainfuckCompiler => {
                panic!("{} is a powdr only benchmark", args.program.to_string())
            }
            _ => SP1Stdin::new(),
        };

        // Get the elf.
        let elf_path = get_elf(args);
        let elf = fs::read(elf_path).unwrap();

        let prover = SP1Prover::<DefaultProverComponents>::new();

        // Setup the program.
        let ((pk, vk), setup_duration) = time_operation(|| prover.setup(&elf));

        // Execute the program.
        let context = SP1Context::default();
        let ((_, report), execution_duration) =
            time_operation(|| prover.execute(&elf, &stdin, context.clone()).unwrap());

        let cycles = report.total_instruction_count();

        // Setup the prover opionts.
        let opts = Default::default();

        // Generate the core proof.
        let (core_proof, prove_core_duration) =
            time_operation(|| prover.prove_core(&pk, &stdin, opts, context).unwrap());

        let num_shards = core_proof.proof.0.len();

        // Verify the proof.
        let core_bytes = bincode::serialize(&core_proof).unwrap();
        let (_, verify_core_duration) = time_operation(|| {
            prover.verify(&core_proof.proof, &vk).expect("Proof verification failed")
        });

        let (compress_proof, compress_duration) =
            time_operation(|| prover.compress(&vk, core_proof, vec![], opts).unwrap());

        let compress_bytes = bincode::serialize(&compress_proof).unwrap();

        let prove_duration = prove_core_duration + compress_duration;

        // Create the performance report.
        PerformanceReport {
            shards: num_shards,
            cycles: cycles as u64,
            speed: (cycles as f64) / prove_core_duration.as_secs_f64(),
            execution_duration: execution_duration.as_secs_f64(),
            prove_duration: prove_duration.as_secs_f64(),
            core_prove_duration: prove_core_duration.as_secs_f64(),
            core_verify_duration: verify_core_duration.as_secs_f64(),
            core_proof_size: core_bytes.len(),
            core_speed: (cycles as f64) / prove_core_duration.as_secs_f64(),
            compress_prove_duration: compress_duration.as_secs_f64(),
            compress_verify_duration: 0.0, // TODO: fill this in.
            compress_proof_size: compress_bytes.len(),
            setup_duration: setup_duration.as_secs_f64(),
        }
    }
}
