extern crate alloc;
use std::collections::VecDeque;

pub fn run(program: Vec<u32>, mut inputs: VecDeque<u32>) -> (u64, Vec<u8>) {
    let mut pc: usize = 0;
    let mut data_ptr: usize = 0;
    let mut loop_stack: Vec<usize> = Vec::new();
    let mut memory = vec![0i64; 30000];

    let mut output = vec![];

    let mut instr_count = 0;
    loop {
        let op = program[pc];

        if op == 0 {
            break;
        }

        instr_count += 1;

        if op == 62 {
            data_ptr += 1;
        } else if op == 60 {
            data_ptr -= 1;
        } else if op == 43 {
            memory[data_ptr] += 1;
        } else if op == 45 {
            memory[data_ptr] -= 1;
        } else if op == 44 {
            // read -1 on EOF
            memory[data_ptr] = inputs.pop_front().map(|i| i as i64).unwrap_or(-1);
        } else if op == 46 {
            output.push(memory[data_ptr] as u8);
        } else if op == 91 {
            if memory[data_ptr] == 0 {
                let mut depth = 1;
                while depth != 0 {
                    pc += 1;
                    if program[pc] == 91 {
                        depth += 1;
                    } else if program[pc] == 93 {
                        depth -= 1;
                    }
                }
            } else {
                loop_stack.push(pc);
            }
        } else if op == 93 {
            pc = loop_stack.pop().unwrap() - 1;
        }

        pc += 1;
    }

    (instr_count, output)
}
