#![feature(iter_intersperse)]

use std::convert::TryFrom;

use crate::bf::ExecutionState;

pub mod bf;
pub mod generate;

fn step_count(program: &bf::Program, max_steps: usize) -> Option<usize> {
    let mut ctx = bf::ExecutionContext::new(program);
    let mut total_real_steps = 0;
    for _ in 1..max_steps {
        let (real_steps, state) = ctx.step();
        total_real_steps += real_steps;
        match state {
            ExecutionState::Halted => {
                return Some(total_real_steps);
            }
            ExecutionState::InfiniteLoop => {
                // eprintln!("bailed: {} (infinite loop)", program);
                return None;
            }
            ExecutionState::Running => (),
        }
    }
    eprintln!("bailed: {} (timeout)", program);
    None
}

fn beaver(length: usize, max_steps: usize) -> (Vec<bf::Program>, usize) {
    let mut best_programs = vec![];
    let mut best_steps = 0;
    let programs = generate::brute_force_iterator(length);
    for program in programs {
        match step_count(&program, max_steps) {
            Some(steps) if steps > best_steps => {
                best_programs = vec![program];
                best_steps = steps;
            }
            Some(steps) if steps == best_steps => {
                best_programs.push(program);
            }
            Some(_) => (),
            None => (),
        }
    }
    (best_programs, best_steps)
}

fn trace(program: &bf::Program) {
    let mut ctx = bf::ExecutionContext::new(program);
    ctx.print_state();
    loop {
        let (_, state) = ctx.step();
        ctx.print_state();
        match state {
            ExecutionState::Running => (),
            ExecutionState::Halted => {
                println!("Halted.");
                break;
            }
            ExecutionState::InfiniteLoop => {
                println!("Infinite loop detected.");
                break;
            }
        }
    }
}

fn main() {
    let max_steps = 1_000_000;
    let debug = false;
    if debug {
        let program = bf::Program::try_from("++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.").unwrap();
        println!("{:?}", program);
        trace(&program);
        println!("{:?}", step_count(&program, max_steps));
    } else {
        for i in 0..6 {
            let (programs, steps) = beaver(i, max_steps);

            println!(
            "Best Programs for Beaver (length = {}, steps = {} or best runs for longer than {})",
            i, steps, max_steps
        );
            for program in programs {
                println!("{}", program);
            }
        }
    }
}
