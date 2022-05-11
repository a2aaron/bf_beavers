use std::convert::TryFrom;

use bf_beavers::{bf, generate};

use crate::bf::{ExecutionState, LoopReason};

fn step_count(program: &bf::Program, max_steps: usize) -> Option<usize> {
    let mut ctx = bf::ExecutionContext::new(program);
    let mut total_real_steps = 0;
    for _ in 1..max_steps {
        let (real_steps, state) = ctx.step();
        total_real_steps += real_steps;
        match state {
            ExecutionState::Halted => {
                eprintln!("HALT:  {}", program);
                return Some(total_real_steps);
            }
            ExecutionState::InfiniteLoop(loop_reason) => {
                let msg = match loop_reason {
                    LoopReason::LoopIfNonzero => "LOOP1",
                    LoopReason::LoopSpan { .. } => "LOOP2",
                };
                eprintln!("{}: {}", msg, program);
                return None;
            }
            ExecutionState::Running => (),
        }
    }
    eprintln!("TIME:  {}", program);
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

    println!(
        "ratio = {}/{}",
        generate::brute_force_iterator(length).count(),
        generate::lexiographic_order(length).count(),
    );
    (best_programs, best_steps)
}

fn trace(program: &bf::Program, max_steps: usize) {
    let mut ctx = bf::ExecutionContext::new(program);

    for _ in 0..max_steps {
        ctx.print_state(true);
        println!("---");
        let (_, state) = ctx.step();

        match state {
            ExecutionState::Running => (),
            ExecutionState::Halted => {
                println!("Halted.");
                break;
            }
            ExecutionState::InfiniteLoop(loop_reason) => {
                ctx.print_state(true);
                println!("Infinite loop detected. Reason: {}", loop_reason);
                break;
            }
        }
    }
}

fn main() {
    let max_steps = 5_000;
    let debug = false;
    if debug {
        let program = bf::Program::try_from("+[>+]").unwrap();
        println!("{:?}", program);
        trace(&program, max_steps);
        println!("{:?}", step_count(&program, max_steps));
    } else {
        for i in 0..8 {
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
