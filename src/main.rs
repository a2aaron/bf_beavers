#![feature(iter_intersperse)]

use std::convert::TryFrom;

pub mod bf;
pub mod generate;

fn step_count(program: &bf::Program, max_steps: usize) -> Option<usize> {
    let mut ctx = bf::ExecutionContext::new(program);
    for steps in 1..max_steps {
        ctx.step();
        if ctx.halted() {
            return Some(steps);
        }
    }
    eprintln!("bailed: {}", program);
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
        ctx.step();
        ctx.print_state();
        if ctx.halted() {
            break;
        }
    }
}

fn main() {
    // let program = bf::Program::try_from("+++---").unwrap();
    // trace(&program);
    // println!("{:?}", program);
    // println!("{:?}", step_count(&program, 1_000_000));
    let max_steps = 1_000_000;
    for i in 0..10 {
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
