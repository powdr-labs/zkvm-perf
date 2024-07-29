use std::path::PathBuf;

use crate::HashFnId;
use crate::{
    get_elf, time_operation, EvalArgs, PerformanceReport, PerformanceReportGenerator, ProgramId,
};

use powdr_number::{Bn254Field, FieldElement, GoldilocksField};

pub struct PowdrPerformanceReportGenerator {}

fn compile_program<F: FieldElement>(program: &ProgramId) -> Option<(PathBuf, String)> {
    println!("compiling {}...", program.to_string());

    let program = format!("../programs/{}", program.to_string());
    let output_dir: PathBuf = format!("/tmp/").into();
    let force_overwrite = true;
    let runtime = powdr_riscv::Runtime::base();
    let via_elf = true;
    let with_bootloader = false;

    let res = powdr_riscv::compile_rust::<F>(
        program.as_str(),
        &output_dir,
        force_overwrite,
        &runtime,
        via_elf,
        with_bootloader,
    );
    res
}

impl PerformanceReportGenerator for PowdrPerformanceReportGenerator {
    fn get_report(args: &EvalArgs) -> PerformanceReport {
        // generate powdr asm
        let (path, asm) = compile_program::<GoldilocksField>(&args.program).unwrap();

        // run powdr pipeline
        let force_overwrite = true;
        let backend = powdr_pipeline::BackendType::Plonky3Composite;
        let mut pipeline = powdr_pipeline::Pipeline::<GoldilocksField>::default()
            .from_asm_string(asm, Some(path))
            .with_output("/tmp/".into(), force_overwrite)
            .with_prover_inputs(vec![])
            // .with_setup_file()
            .with_backend(backend, Some("stark_gl".into()))
            .with_pil_object();

        let (witness, witgen_time) = time_operation(|| pipeline.compute_witness().unwrap());
        println!("witgen time: {:?}", witgen_time);
        let (proof, proof_time) = time_operation(|| pipeline.compute_proof().unwrap());
        println!("proof time: {:?}", proof_time);

        panic!()
    }
}
