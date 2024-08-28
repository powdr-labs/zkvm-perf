import os
import subprocess
from itertools import product

filename = 'benchmark'
trials = 1
options_program = ['loop', 'fibonacci', 'tendermint', 'keccak', 'brainfuck', 'brainfuck-asm', 'brainfuck-compiler']
options_prover = ['sp1', 'risc0', 'powdr-plonky3']
options_hashfn = ['poseidon']
options_shard_size = [20]

powdr_only_programs = ['brainfuck-compiler', 'brainfuck-asm']
args_brainfuck = ['programs/brainfuck/rot13.bf', 'programs/brainfuck/rot13.in']

## for reth, use these
# options_program = ['reth1', 'reth2']
# options_block_1 = '17106222'
# options_block_2 = '19409768'



option_combinations = product(options_program, options_prover, options_hashfn, options_shard_size)
for program, prover, hashfn, shard_size in option_combinations:
    first_shard_size = options_shard_size[0]
    # TODO: check if this is needed, as risc0 has segment_limit_po2
    # if prover not in ['sp1', 'powdr-plonky3'] and shard_size != first_shard_size:
    #     continue
    if not prover.startswith('powdr') and program in powdr_only_programs:
        print(f'Skipping: {program}, {prover}, {hashfn}, {shard_size} (program not supported by prover)')
        continue
    print(f'Running: {program}, {prover}, {hashfn}, {shard_size}')
    env = os.environ.copy();
    env['MAX_DEGREE_LOG'] = str(shard_size)
    for _ in range(trials):
        if program == 'reth1':
            subprocess.run(['bash', 'eval.sh', 'reth', prover, hashfn, str(shard_size), filename, options_block_1], env=env)
        elif program == 'reth2':
            subprocess.run(['bash', 'eval.sh', 'reth', prover, hashfn, str(shard_size), filename, options_block_2], env=env)
        elif program.startswith('brainfuck'):
            subprocess.run(['bash', 'eval.sh', program, prover, hashfn, str(shard_size), filename, args_brainfuck[0], args_brainfuck[1]], env=env)
        else:
            subprocess.run(['bash', 'eval.sh', program, prover, hashfn, str(shard_size), filename], env=env)
