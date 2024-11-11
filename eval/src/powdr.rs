use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use powdr_number::{FieldElement, GoldilocksField, KnownField};
use powdr_pipeline::Pipeline;

use cfg_if::cfg_if;

use crate::{utils::*, EvalArgs, HashFnId, PerformanceReport, ProgramId, ProverId};

// TODO: build to some other directory?
const OUTPUT_DIR: &str = "/tmp";

pub struct PowdrEvaluator;

fn run<T: FieldElement>(mut pipeline: powdr_pipeline::Pipeline<T>) -> PerformanceReport {
    println!("running powdr with no continuations...");
    // pre-compute fixed cols
    pipeline.compute_fixed_cols().expect("error generating fixed columns");
    // compute witness
    let start = Instant::now();
    pipeline.compute_witness().unwrap();
    let witgen_time = start.elapsed();
    // TODO: we're not proving RISCV programs, so "cycles" is not a thing necessarily
    let trace_len = {
        let cols = pipeline.witness().unwrap();
        cols.iter()
            .filter(|(name, _)| name == "main::pc")
            .map(|(_, col)| col.len() as u64)
            .next()
            .unwrap()
    };

    let (_, setup_duration) =
        time_operation(|| pipeline.setup_backend().expect("could not setup the backend"));

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
        shards: 1,
        cycles: trace_len,
        execution_duration: witgen_time.as_secs_f64(),
        core_prove_duration: core_proof_duration.as_secs_f64(),
        core_verify_duration: core_verification_time.as_secs_f64(),
        core_proof_size,
        core_speed: (trace_len as f64) / core_proof_duration.as_secs_f64(),
        setup_duration: setup_duration.as_secs_f64(),
        // TODO: we don't do recursion/compression yet, so these are all 0
        compress_prove_duration: 0.0,
        compress_verify_duration: 0.0,
        compress_proof_size: 0,
        speed: 0.0,
        prove_duration: 0.0,
    }
}

fn run_with_continuations<T: FieldElement>(
    mut pipeline: powdr_pipeline::Pipeline<T>,
) -> PerformanceReport {
    println!("running powdr with continuations...");
    // pre-compute fixed columns
    pipeline.compute_fixed_cols().expect("error generating fixed columns");

    // execute with continuations
    println!("continuations dry run...");
    let start = Instant::now();
    let dry_run = powdr_riscv::continuations::rust_continuations_dry_run(&mut pipeline, None);

    let num_chunks = dry_run.bootloader_inputs.len();
    let trace_len = dry_run.trace_len as u64;

    let generate_witness = |pipeline: &mut Pipeline<T>| -> Result<(), Vec<String>> {
        pipeline.compute_witness().unwrap();
        Ok(())
    };
    // this will save the witness for each chunk N in its own `chunk_N` directory
    println!("continuations witgen...");
    powdr_riscv::continuations::rust_continuations(&mut pipeline, generate_witness, dry_run)
        .expect("error executing with continuations");
    let witgen_time = start.elapsed();
    println!("continuations witgen time: {witgen_time:?}");

    let (_, setup_duration) =
        time_operation(|| pipeline.setup_backend().expect("could not setup the backend"));

    // compute proof for each chunk
    let mut core_proof_duration = Duration::default();
    let mut core_proof_size = 0;
    let mut proofs = vec![];
    println!("proving chunks...");
    for chunk in 0..num_chunks {
        let witness_dir: PathBuf = format!("{OUTPUT_DIR}/chunk_{chunk}").into();
        pipeline = pipeline.read_witness(&witness_dir).unwrap().with_output(witness_dir, true);
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
        shards: num_chunks,
        cycles: trace_len,
        execution_duration: witgen_time.as_secs_f64(),
        core_prove_duration: core_proof_duration.as_secs_f64(),
        core_verify_duration: core_verification_time.as_secs_f64(),
        core_proof_size,
        core_speed: (trace_len as f64) / core_proof_duration.as_secs_f64(),
        setup_duration: setup_duration.as_secs_f64(),
        // TODO: we don't do recursion/compression yet, so these are all 0
        compress_prove_duration: 0.0,
        compress_verify_duration: 0.0,
        compress_proof_size: 0,
        speed: 0.0,
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
                compile_program::<GoldilocksField>(path, args.shard_size, true).unwrap()
            }
            ProgramId::Reth => {
                let path = format!("programs/{}-powdr", args.program.to_string());
                compile_program::<GoldilocksField>(path, args.shard_size, true).unwrap()
            }
            ProgramId::BrainfuckAsm => {
                let path = format!("programs/brainfuck/brainfuck_vm.asm").into();
                let asm =
                    std::fs::read_to_string(&path).expect("error reading brainfuck powdr asm file");
                (Some(path), asm)
            }
            ProgramId::BrainfuckCompiler => {
                let (program, _) = get_brainfuck_input(args);
                let bf_asm = compile_brainfuck(&program[..]);
                let path = "programs/brainfuck/brainfuck_isa.asm";
                let bf_isa = std::fs::read_to_string(path).unwrap();
                let bf_vm = bf_isa.replace("{{ program }}", bf_asm.as_str());
                println!("{bf_vm}");
                (Some(path.into()), bf_vm)
            }
            program => {
                let path = format!("programs/{}", program.to_string());
                compile_program::<GoldilocksField>(
                    path,
                    args.shard_size,
                    !args.powdr_no_continuations,
                )
                .unwrap()
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
                let backend = powdr_pipeline::BackendType::Plonky3;
            } else {
                unreachable!();
            }
        }
        let mut pipeline = powdr_pipeline::Pipeline::<GoldilocksField>::default()
            .from_asm_string(asm, path)
            .with_output(dir.into(), true)
            .with_prover_inputs(vec![])
            // .with_setup_file()
            // .with_pil_object()
            .with_backend(backend, None);

        // set program inputs
        match args.program {
            ProgramId::Brainfuck => {
                let (program, input) = get_brainfuck_input(args);
                pipeline = pipeline.add_data(0, &program).add_data(1, &input)
            }
            ProgramId::BrainfuckAsm => {
                let (program, input) = get_brainfuck_input(args);
                let prover_inputs = std::iter::once(program.len() as u32)
                    .chain(program.into_iter())
                    .chain(std::iter::once(input.len() as u32))
                    .chain(input)
                    .map(|n| n.into())
                    .collect();
                pipeline = pipeline.with_prover_inputs(prover_inputs);
            }
            ProgramId::BrainfuckCompiler => {
                let (_, input) = get_brainfuck_input(args);
                let prover_inputs =
                    std::iter::once(input.len() as u32).chain(input).map(|n| n.into()).collect();
                pipeline = pipeline.with_prover_inputs(prover_inputs);
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
            ProgramId::BrainfuckAsm | ProgramId::BrainfuckCompiler => run(pipeline),
            _ => {
                if args.powdr_no_continuations {
                    run(pipeline)
                } else {
                    run_with_continuations(pipeline)
                }
            }
        }
    }
}

fn compile_program<F: FieldElement>(
    crate_path: String,
    shard_size: u64,
    with_continuations: bool,
) -> Option<(Option<PathBuf>, String)> {
    println!("compiling {} (continuations={with_continuations})...", crate_path.to_string());

    // we shift it by 2 (i.e., multiply by 4) because, in powdr,
    // there is a division by 4 to get the trace that fits inside a chunk (due to the
    // expectation of a memory machine 4x larger than main)
    let max_degree_log = shard_size + 2;

    let output_dir: PathBuf = OUTPUT_DIR.into();
    let force_overwrite = true;
    let known_field = F::known_field().unwrap();
    let options = match known_field {
        KnownField::GoldilocksField => {
            let opt =
                powdr_riscv::CompilerOptions::new_gl().with_max_degree_log(max_degree_log as u8);
            if with_continuations {
                opt.with_continuations()
            } else {
                opt
            }
        }
        _ => {
            todo!()
        }
    };

    let res = powdr_riscv::compile_rust(
        crate_path.as_str(),
        options,
        &output_dir,
        force_overwrite,
        // enable powdr feature on compiled program
        Some(vec!["powdr".to_string()]),
    );
    res.map(|(path, asm)| (Some(path), asm))
}
