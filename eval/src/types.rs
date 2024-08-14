/// An identifier used to select the program to evaluate.
#[derive(clap::ValueEnum, Clone, PartialEq)]
#[clap(rename_all = "kebab_case")]
pub enum ProgramId {
    Loop,
    Fibonacci,
    SSZWithdrawals,
    Tendermint,
    Sha2Chain,
    Reth,
    Keccak,
    // brainfuck interpreter in Rust
    Brainfuck,
    // brainfuck interpreter handwritten in Powdr ASM
    BrainfuckAsm,
    // brainfuck compiled into Powdr ASM
    BrainfuckCompiler,
}

/// An identifier used to select the prover to evaluate.
#[derive(clap::ValueEnum, Clone, PartialEq)]
pub enum ProverId {
    Risc0,
    SP1,
    JoltZkvm,
    PowdrEstark,
    PowdrPlonky3,
}

/// An identifier used to select the hash function to evaluate.
#[derive(clap::ValueEnum, Clone, PartialEq)]
pub enum HashFnId {
    Sha256,
    Poseidon,
    Blake3,
    Keccak256,
}

impl ProgramId {
    /// Convert the identifier to a string.
    pub fn to_string(&self) -> String {
        match self {
            ProgramId::Loop => "loop".to_string(),
            ProgramId::Fibonacci => "fibonacci".to_string(),
            ProgramId::SSZWithdrawals => "ssz-withdrawals".to_string(),
            ProgramId::Tendermint => "tendermint".to_string(),
            ProgramId::Sha2Chain => "sha2-chain".to_string(),
            ProgramId::Reth => "reth".to_string(),
            ProgramId::Keccak => "keccak".to_string(),
            ProgramId::Brainfuck => "brainfuck".to_string(),
            ProgramId::BrainfuckAsm => "brainfuck-asm".to_string(),
            ProgramId::BrainfuckCompiler => "brainfuck-compiler".to_string(),
        }
    }
}

impl ProverId {
    /// Convert the identifier to a string.
    pub fn to_string(&self) -> String {
        match self {
            ProverId::Risc0 => "risc0".to_string(),
            ProverId::SP1 => "sp1".to_string(),
            ProverId::JoltZkvm => "jolt-zkvm".to_string(),
            ProverId::PowdrEstark => "powdr-estark".to_string(),
            ProverId::PowdrPlonky3 => "powdr-plonky3".to_string(),
        }
    }
}

impl HashFnId {
    /// Convert the identifier to a string.
    pub fn to_string(&self) -> String {
        match self {
            HashFnId::Sha256 => "sha-256".to_string(),
            HashFnId::Poseidon => "poseidon".to_string(),
            HashFnId::Blake3 => "blake3".to_string(),
            HashFnId::Keccak256 => "keccak256".to_string(),
        }
    }
}
