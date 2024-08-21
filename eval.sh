#!/bin/bash
set -e
echo "Running $1, $2, $3, $4, $5"

# if there's a specific crate for the prover, use it
program_dir="programs/${1}"
if [ -d "${program_dir}-${2}" ]; then
    program_dir="${program_dir}-${2}"
fi

# compile the program if SP1 or RISC0 (powdr is compiled inside `eval`)
if [ "$2" == "sp1" ]; then
    echo "Building program for SP1"
    pushd "${program_dir}"
    # The reason we don't just use `cargo prove build` from the SP1 CLI is we need to pass a --features ...
    # flag to select between sp1 and risc0.
    RUSTFLAGS="-C passes=loweratomic -C link-arg=-Ttext=0x00200800 -C panic=abort" \
        RUSTUP_TOOLCHAIN=succinct \
        CARGO_BUILD_TARGET=riscv32im-succinct-zkvm-elf \
        cargo build --release --ignore-rust-version --features $2
    popd
elif [ "$2" == "risc0" ]; then
    echo "Building program for Risc0"
    pushd "${program_dir}"
    # Use the risc0 toolchain.
    RUSTFLAGS="-C passes=loweratomic -C link-arg=-Ttext=0x00200800 -C panic=abort" \
        RUSTUP_TOOLCHAIN=risc0 \
        CARGO_BUILD_TARGET=riscv32im-risc0-zkvm-elf \
        cargo build --release --ignore-rust-version --features $2
    popd
fi

echo "Running eval script"

# Detect whether we're on an instance with a GPU.
if nvidia-smi > /dev/null 2>&1; then
  echo "running on GPU"
  GPU_EXISTS=true
else
  GPU_EXISTS=false
fi

# Determine the features based on GPU existence.
if [ "$GPU_EXISTS" = true ]; then
  FEATURES="cuda"
else
  FEATURES="default"
  if grep -e avx512 /proc/cpuinfo > /dev/null; then
    echo "running with AVX support"
    export RUSTFLAGS='-C target-cpu=native -C target_feature=+avx512ifma,+avx512vl'
    FEATURES="${FEATURES},avx512"
  else
    echo "running with no AVX support"
    export RUSTFLAGS='-C target-cpu=native'
  fi
fi

# Set the logging level.
export RUST_LOG=info

# shard-size is set by MAX_DEGREE_LOG env var in powdr
export MAX_DEGREE_LOG=$4

# Run the benchmark.
cargo run \
    -p sp1-benchmarks-eval \
    --release \
    --no-default-features \
    --features $FEATURES,$2 \
    -- \
    --program $1 \
    --prover $2 \
    --hashfn $3 \
    --shard-size $4 \
    --filename $5 \
    ${@:6}
