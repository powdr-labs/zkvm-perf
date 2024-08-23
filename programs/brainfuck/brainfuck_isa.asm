use std::machines::range::Byte2;
use std::machines::memory::Memory;

machine Brainfuck {
    Byte2 byte2;
    Memory mem(byte2);

    // the program pc
    reg pc[@pc];
    // assignment register used by instruction parameters
    reg X[<=];

    // data pointer
    reg dp;
    // remaining input count
    reg in_remaining;
    let EOF = std::utils::is_zero(in_remaining);
    // total input count
    reg in_count;
    // helper data container
    reg data;

    // iszero check for X
    let XIsZero = std::utils::is_zero(X);

    // instructions needed for Brainfuck operations

    instr branch_if_zero X, l: label
    {
        pc' = XIsZero * l + (1 - XIsZero) * (pc + 1)
    }

    instr jump l: label{ pc' = l }

    instr fail { 1 = 0 }

    instr inc_dp { dp' = dp + 1 }
    instr dec_dp { dp' = dp - 1 }

    // helper column
    col witness C;

    col witness Input(unused) query Query::Input(to_int(std::prover::eval(in_count - in_remaining) + 1) % (to_int(std::prover::eval(in_count)) + 1));

    // read will store a -1 on EOF
    instr read
        link if (1 - EOF) ~> mem.mstore(dp, STEP, Input)
        link if EOF ~> mem.mstore(dp, STEP, -1)
    {
        in_remaining' = in_remaining - (1 - EOF)
    }

    instr inc_cell
       link ~> C = mem.mload(dp, STEP)
       link ~> mem.mstore(dp, STEP, C + 1);

    instr dec_cell
       link ~> C = mem.mload(dp, STEP)
       link ~> mem.mstore(dp, STEP, C - 1);

    // memory instructions
    col fixed STEP(i) { i };
    instr mload -> X
        link ~> X = mem.mload(dp, STEP);

    instr mstore X
        link ~> mem.mstore(dp, STEP, X);

    function main {
        // we expect Query::Input(0) to be the number of inputs
        in_count <=X= ${ Query::Input(0) };
        in_remaining <=X= in_count;
        // compiled Brainfuck program
        {{ program }}
    }
}
