#![feature(iter_intersperse)]

use std::convert::TryFrom;

use crate::bf::ExecutionContext;

pub mod bf;

fn main() {
    let program = bf::Program::try_from("-[-]").unwrap();
    println!("{}", program);

    let mut ctx = ExecutionContext::new(program);
    while !ctx.halted() {
        ctx.print_state();
        ctx.step();
    }
}
