use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use powdr_number::{FieldElement, GoldilocksField};
use powdr_pipeline::Pipeline;

use cfg_if::cfg_if;

use crate::{utils::*, EvalArgs, HashFnId, PerformanceReport, ProgramId, ProverId};

// TODO: build to some other directory?
const OUTPUT_DIR: &str = "/tmp";

pub struct PowdrEvaluator;

fn run<T: FieldElement>(
    args: &EvalArgs,
    mut pipeline: powdr_pipeline::Pipeline<T>,
) -> PerformanceReport {
    println!("running powdr with no continuations...");
    // pre-compute fixed cols
    pipeline.compute_fixed_cols().expect("error generating fixed columns");
    // compute witness
    let start = Instant::now();
    pipeline.compute_witness().unwrap();
    let witgen_time = start.elapsed();
    // TODO: trace_len
    let trace_len = 0;
    // compute proof
    let start = Instant::now();
    pipeline.compute_proof().unwrap();
    let core_proof_duration = start.elapsed();
    let proof = pipeline.proof().unwrap();
    let core_proof_size = proof.len();
    // verify
    let mut pipeline = pipeline.clone();
    {
        let mut writer = std::fs::File::create(format!("{OUTPUT_DIR}/vkey.bin")).unwrap();
        pipeline.export_verification_key(&mut writer).unwrap();
    }
    let (_, core_verification_time) = time_operation(|| {
        pipeline.verify(proof, &[vec![]]).unwrap();
    });

    PerformanceReport {
        // The program that is being evaluated.
        program: args.program.to_string(),
        // The prover that is being evaluated.
        prover: args.prover.to_string(),
        // The hash function that is being evaluated.
        hashfn: args.hashfn.to_string(),
        // The shard size that is being evaluated.
        shard_size: args.shard_size,
        // The number of shards.
        shards: 1,
        // The reported number of cycles.
        cycles: trace_len,
        // The reported duration of the execution in seconds.
        execution_duration: witgen_time.as_secs_f64(),
        // The reported duration of the core proving time in seconds.
        core_prove_duration: core_proof_duration.as_secs_f64(),
        // The reported duration of the verifier in seconds.
        core_verify_duration: core_verification_time.as_secs_f64(),
        // The size of the core proof.
        core_proof_size,
        core_speed: (trace_len as f64) / core_proof_duration.as_secs_f64(),

        // TODO: we don't do recursion/compression yet, so these are all 0

        // The reported duration of the recursive proving time in seconds.
        compress_prove_duration: 0.0,
        // The reported duration of the verifier in seconds.
        compress_verify_duration: 0.0,
        // The size of the recursive proof in bytes.
        compress_proof_size: 0,
        // The reported speed in cycles per second.
        speed: 0.0,
        // The reported duration of the prover in seconds.
        prove_duration: 0.0,
    }
}

fn run_with_continuations<T: FieldElement>(
    args: &EvalArgs,
    mut pipeline: powdr_pipeline::Pipeline<T>,
) -> PerformanceReport {
    println!("running powdr with continuations...");
    // pre-compute fixed columns
    pipeline.compute_fixed_cols().expect("error generating fixed columns");

    // execute with continuations
    println!("continuations dry run...");
    let start = Instant::now();
    let bootloader_inputs =
        powdr_riscv::continuations::rust_continuations_dry_run(&mut pipeline, None);

    // TODO: is this the correct trace size?
    let trace_len: u64 = bootloader_inputs.iter().map(|(_, n)| n).sum();
    let num_chunks = bootloader_inputs.len();
    println!("trace length: {trace_len}");

    let generate_witness = |mut pipeline: Pipeline<T>| -> Result<(), Vec<String>> {
        pipeline.compute_witness().unwrap();
        Ok(())
    };
    // this will save the witness for each chunk N in its own `chunk_N` directory
    println!("continuations witgen...");
    powdr_riscv::continuations::rust_continuations(
        pipeline.clone(),
        generate_witness,
        bootloader_inputs,
    )
    .expect("error executing with continuations");
    let witgen_time = start.elapsed();
    println!("continuations witgen time: {witgen_time:?}");

    // // load computed fixed cols
    // println!("read fixed columns...");
    // let mut pipeline = pipeline.read_constants(OUTPUT_DIR.as_ref());

    // compute proof for each chunk
    let mut core_proof_duration = Duration::default();
    let mut core_proof_size = 0;
    let mut proofs = vec![];
    println!("proving chunks...");
    for chunk in 0..num_chunks {
        let witness_dir: PathBuf = format!("{OUTPUT_DIR}/chunk_{chunk}").into();
        let mut pipeline =
            pipeline.clone().read_witness(&witness_dir).with_output(witness_dir, true);
        let (proof, chunk_duration) = time_operation(|| pipeline.compute_proof().unwrap().clone());
        println!("chunk {chunk} proof time: {chunk_duration:?}");
        let chunk_size = proof.len();
        proofs.push(proof);
        println!("chunk size: {chunk_size}");
        core_proof_duration += chunk_duration;
        core_proof_size += chunk_size;
    }
    println!("total proof time: {core_proof_duration:?}");
    println!("total proof size: {core_proof_size}");

    // verify each chunk
    let mut core_verification_time = Duration::default();
    println!("exporting verification key...");
    {
        let mut writer = std::fs::File::create(format!("{OUTPUT_DIR}/vkey.bin")).unwrap();
        pipeline.export_verification_key(&mut writer).unwrap();
    }
    println!("verifying chunks...");
    for chunk in 0..num_chunks {
        let (_, time) = time_operation(|| {
            pipeline.verify(&proofs[chunk], &[vec![]]).unwrap();
        });
        println!("chunk {chunk} verification time: {time:?}");
        core_verification_time += time;
    }

    PerformanceReport {
        // The program that is being evaluated.
        program: args.program.to_string(),
        // The prover that is being evaluated.
        prover: args.prover.to_string(),
        // The hash function that is being evaluated.
        hashfn: args.hashfn.to_string(),
        // The shard size that is being evaluated.
        shard_size: args.shard_size,
        // The number of shards.
        shards: num_chunks,
        // The reported number of cycles.
        cycles: trace_len,
        // The reported duration of the execution in seconds.
        execution_duration: witgen_time.as_secs_f64(),
        // The reported duration of the core proving time in seconds.
        core_prove_duration: core_proof_duration.as_secs_f64(),
        // The reported duration of the verifier in seconds.
        core_verify_duration: core_verification_time.as_secs_f64(),
        // The size of the core proof.
        core_proof_size,
        core_speed: (trace_len as f64) / core_proof_duration.as_secs_f64(),

        // TODO: we don't do recursion/compression yet, so these are all 0

        // The reported duration of the recursive proving time in seconds.
        compress_prove_duration: 0.0,
        // The reported duration of the verifier in seconds.
        compress_verify_duration: 0.0,
        // The size of the recursive proof in bytes.
        compress_proof_size: 0,
        // The reported speed in cycles per second.
        speed: 0.0,
        // The reported duration of the prover in seconds.
        prove_duration: 0.0,
    }
}

impl PowdrEvaluator {
    pub fn eval(args: &EvalArgs) -> PerformanceReport {
        assert!(args.hashfn == HashFnId::Poseidon);

        // setup logger
        let mut builder = env_logger::Builder::new();
        builder.parse_default_env().target(env_logger::Target::Stdout).init();

        // generate powdr asm
        let (path, asm) = match &args.program {
            ProgramId::Tendermint => {
                let path = format!("programs/{}-powdr", args.program.to_string());
                compile_program::<GoldilocksField>(path, true).unwrap()
            }
            ProgramId::Reth => {
                let path = format!("programs/{}-powdr", args.program.to_string());
                compile_program::<GoldilocksField>(path, true).unwrap()
            }
            ProgramId::BrainfuckAsm => {
                let path = format!("programs/brainfuck/brainfuck.asm").into();
                let asm =
                    std::fs::read_to_string(&path).expect("error reading brainfuck powdr asm file");
                (path, asm)
            }
            ProgramId::BrainfuckCompiler => {
                todo!()
            }
            program => {
                let path = format!("programs/{}", program.to_string());
                compile_program::<GoldilocksField>(path, !args.powdr_no_continuations).unwrap()
            }
        };

        let dir = "/tmp";

        // build the powdr pipeline
        cfg_if! {
            if #[cfg(feature = "powdr-estark")] {
                println!("using EStark backend");
                assert!(args.prover == ProverId::PowdrEstark);
                let backend = powdr_pipeline::BackendType::EStarkPolygonComposite;
            } else if #[cfg(feature = "powdr-plonky3")] {
                println!("using Plonky3 backend");
                assert!(args.prover == ProverId::PowdrPlonky3);
                let backend = powdr_pipeline::BackendType::Plonky3Composite;
            } else {
                unreachable!();
            }
        }
        let mut pipeline = powdr_pipeline::Pipeline::<GoldilocksField>::default()
            .from_asm_string(asm, Some(path))
            .with_output(dir.into(), true)
            .with_prover_inputs(vec![])
            // .with_setup_file()
            // .with_pil_object()
            .with_backend(backend, None);

        // set program inputs
        match args.program {
            ProgramId::Brainfuck | ProgramId::BrainfuckAsm | ProgramId::BrainfuckCompiler => {
                let (program, input) = get_brainfuck_input(args);
                pipeline = pipeline.add_data(0, &program).add_data(1, &input)
            }
            ProgramId::Reth => {
                let data = vec![(0, get_reth_input(args))];
                pipeline = pipeline.add_data_vec(&data[..]);
            }
            _ => {}
        }

        // run the pipeline
        match args.program {
            // non-riscv programs can't run with continuations
            ProgramId::BrainfuckAsm | ProgramId::BrainfuckCompiler => run(args, pipeline),
            _ => {
                if args.powdr_no_continuations {
                    run(args, pipeline)
                } else {
                    run_with_continuations(args, pipeline)
                }
            }
        }
    }
}

fn compile_program<F: FieldElement>(
    crate_path: String,
    with_continuations: bool,
) -> Option<(PathBuf, String)> {
    println!("compiling {} (continuations={with_continuations})...", crate_path.to_string());

    let output_dir: PathBuf = OUTPUT_DIR.into();
    let force_overwrite = true;
    let runtime = if with_continuations {
        powdr_riscv::Runtime::base().with_poseidon_for_continuations()
    } else {
        powdr_riscv::Runtime::base().with_poseidon_no_continuations()
    };
    let via_elf = true;

    let res = powdr_riscv::compile_rust::<F>(
        crate_path.as_str(),
        &output_dir,
        force_overwrite,
        &runtime,
        via_elf,
        with_continuations,
        // enable powdr feature on compiled program
        Some(vec!["powdr".to_string()]),
    );
    res
}
