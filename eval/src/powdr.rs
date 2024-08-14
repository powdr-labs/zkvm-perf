use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use powdr_number::{FieldElement, GoldilocksField};
use powdr_pipeline::Pipeline;

use cfg_if::cfg_if;

use crate::{utils::*, EvalArgs, HashFnId, PerformanceReport, ProgramId, ProverId};

pub struct PowdrEvaluator;

impl PowdrEvaluator {
    pub fn eval(args: &EvalArgs) -> PerformanceReport {
        assert!(args.hashfn == HashFnId::Poseidon);

        // generate powdr asm
        let (path, asm) = compile_program::<GoldilocksField>(&args.program).unwrap();

        let dir = "/tmp";

        // build the powdr pipeline
        let force_overwrite = true;
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
            .with_output(dir.into(), force_overwrite)
            .with_prover_inputs(vec![])
            // .with_setup_file()
            .with_backend(backend, None)
            .with_pil_object();

        // set program inputs
        match args.program {
            ProgramId::Brainfuck => {
                let (program, input) = get_brainfuck_input(args);
                pipeline = pipeline.add_data(0, &program).add_data(1, &input)
            }
            ProgramId::Reth => {
                let data = vec![(0, get_reth_input(args))];
                pipeline = pipeline.add_data_vec(&data[..]);
            }
            _ => {}
        }

        // execute with continuations
        let start = Instant::now();
        let bootloader_inputs =
            powdr_riscv::continuations::rust_continuations_dry_run(&mut pipeline, None);

        // TODO: is this the correct trace size?
        let trace_len: u64 = bootloader_inputs.iter().map(|(_, n)| n).sum();
        let num_chunks = bootloader_inputs.len();
        println!("trace length: {trace_len}");

        let generate_witness =
            |mut pipeline: Pipeline<GoldilocksField>| -> Result<(), Vec<String>> {
                pipeline.compute_witness().unwrap();
                Ok(())
            };
        // this will save the witness for each chunk N in its own `chunk_N` directory
        powdr_riscv::continuations::rust_continuations(
            pipeline.clone(),
            generate_witness,
            bootloader_inputs,
        )
        .expect("error executing with continuations");
        let witgen_time = start.elapsed();
        println!("continuations witgen time: {witgen_time:?}");

        // compute proof for each chunk
        let mut core_proof_duration = Duration::default();
        let mut core_proof_size = 0;
        let mut proofs = vec![];
        for chunk in 0..num_chunks {
            let witness_dir: PathBuf = format!("{dir}/chunk_{}", chunk).into();
            let mut pipeline = pipeline
                .clone()
                .read_witness(&witness_dir)
                .with_output(witness_dir, force_overwrite);
            let (proof, chunk_duration) =
                time_operation(|| pipeline.compute_proof().unwrap().clone());
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
        {
            let mut writer = std::fs::File::create("vkey.bin").unwrap();
            pipeline.export_verification_key(&mut writer).unwrap();
        }
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
}

fn compile_program<F: FieldElement>(program: &ProgramId) -> Option<(PathBuf, String)> {
    println!("compiling {}...", program.to_string());

    let program = match program {
        ProgramId::Tendermint => format!("programs/{}-powdr", program.to_string()),
        ProgramId::Reth => format!("programs/{}-powdr", program.to_string()),
        // ProgramId::Reth => ,
        _ => format!("programs/{}", program.to_string()),
    };

    // TODO: build to some other directory?
    let output_dir: PathBuf = format!("/tmp/").into();
    let force_overwrite = true;
    let runtime = powdr_riscv::Runtime::base().with_poseidon_for_continuations();
    let via_elf = true;
    let with_bootloader = true;

    let res = powdr_riscv::compile_rust::<F>(
        program.as_str(),
        &output_dir,
        force_overwrite,
        &runtime,
        via_elf,
        with_bootloader,
        // enable powdr feature on compiled program
        Some(vec!["powdr".to_string()]),
    );
    res
}
