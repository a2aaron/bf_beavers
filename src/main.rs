#![feature(let_chains)]

use std::{convert::TryFrom, io::Write};

use rayon::prelude::*;

use clap::Parser;

use bf_beavers::{
    bf::{self, ExecutionState},
    generate, visualizer,
};

fn step_count(program: &bf::Program, max_steps: usize) -> (ExecutionState, Option<usize>) {
    let mut ctx = bf::ExecutionContext::new(program);
    let mut total_real_steps = 0;
    for _ in 1..max_steps {
        let (real_steps, state) = ctx.step();
        total_real_steps += real_steps;
        match state {
            ExecutionState::Halted => {
                return (state, Some(total_real_steps));
            }
            ExecutionState::InfiniteLoop(_) => {
                return (state, Some(total_real_steps));
            }
            ExecutionState::Running => (),
        }
    }
    (ExecutionState::Running, None)
}

struct BusyBeaverResults {
    best_programs: Vec<bf::Program>,
    best_steps: usize,
    unknown_programs: Vec<bf::Program>,
    num_halted: usize,
    num_looping: usize,
    lexiographic_size: usize,
}

fn beaver(length: usize, max_steps: usize, print_every: Option<usize>) -> BusyBeaverResults {
    let programs = generate::brute_force_iterator(length);
    let lexiographic_size = 6_usize.pow(length as u32);

    programs
        .enumerate()
        .inspect(|(i, program)| {
            if let Some(print_every) = print_every && i % print_every == 0 {
                eprintln!("{}", program)
            }
        })
        .par_bridge()
        .map(|(_, program)| {
            let (state, step) = step_count(&program, max_steps);
            match state {
                ExecutionState::Running => BusyBeaverResults {
                    best_programs: vec![],
                    best_steps: 0,
                    unknown_programs: vec![program],
                    num_halted: 0,
                    num_looping: 0,
                    lexiographic_size,
                },
                ExecutionState::Halted => BusyBeaverResults {
                    best_programs: vec![program],
                    best_steps: step.unwrap(),
                    unknown_programs: vec![],
                    num_halted: 1,
                    num_looping: 0,
                    lexiographic_size,
                },
                ExecutionState::InfiniteLoop(_) => BusyBeaverResults {
                    best_programs: vec![],
                    best_steps: 0,
                    unknown_programs: vec![],
                    num_halted: 0,
                    num_looping: 1,
                    lexiographic_size,
                },
            }
        })
        .reduce(
            || BusyBeaverResults {
                best_programs: vec![],
                best_steps: 0,
                unknown_programs: vec![],
                num_halted: 0,
                num_looping: 0,
                lexiographic_size,
            },
            |mut a, mut b| BusyBeaverResults {
                best_programs: {
                    if a.best_steps == b.best_steps {
                        a.best_programs.append(&mut b.best_programs);
                        a.best_programs
                    } else if a.best_steps > b.best_steps {
                        a.best_programs
                    } else {
                        b.best_programs
                    }
                },
                best_steps: a.best_steps.max(b.best_steps),
                unknown_programs: {
                    a.unknown_programs.append(&mut b.unknown_programs);
                    a.unknown_programs
                },
                num_halted: a.num_halted + b.num_halted,
                num_looping: a.num_looping + b.num_looping,
                lexiographic_size,
            },
        )
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Interactive mode - run with a BF program to visualize
    #[clap(short, long, value_name = "bf program", allow_hyphen_values = true)]
    interactive: Option<String>,
    /// Interactive mode - start at step n
    #[clap(long, value_name = "steps", default_value_t = 0)]
    start_at: usize,
    /// Simple mode - run a BF program and output the number of steps it took
    #[clap(long, value_name = "bf program", allow_hyphen_values = true)]
    run: Option<String>,
    /// How many steps to run programs for before giving up
    #[clap(long, value_name = "steps", default_value_t = 50_000)]
    max_steps: usize,
    /// Beaver mode - The maximum length of programs to generate
    #[clap(long, value_name = "length", default_value_t = 8)]
    max_length: usize,
    /// Beaver mode - Print the nth program
    #[clap(short, value_name = "n", long)]
    print_every: Option<usize>,
}
fn main() {
    let args = Args::parse();
    if let Some(program) = args.run {
        match bf::Program::try_from(program.as_str()) {
            Ok(program) => {
                let (state, steps) = step_count(&program, args.max_steps);
                match state {
                    ExecutionState::Running => {
                        println!("Timed out (runs longer than {} steps)", args.max_steps)
                    }
                    ExecutionState::Halted => println!("Halts in {} steps", steps.unwrap()),
                    ExecutionState::InfiniteLoop(reason) => {
                        println!(
                            "Does not halt (reason: {:#?}, at step {})",
                            reason,
                            steps.unwrap()
                        )
                    }
                }
            }
            Err(err) => println!("Cannot compile {} (reason: {})", program, err),
        }
    } else if let Some(program) = args.interactive {
        match bf::Program::try_from(program.as_str()) {
            Ok(program) => {
                println!("Visualizing {}", program);
                visualizer::run(&program, args.start_at);
                println!("Exiting...");
            }
            Err(err) => println!("Cannot compile {} (reason: {})", program, err),
        }
    } else {
        for i in 0..=args.max_length {
            let mut f = std::fs::File::create(format!("length_{}.txt", i)).unwrap();
            let results = beaver(i, args.max_steps, args.print_every);

            writeln!(f,
                "Best Busy Beavers for Length {}\nTotal steps: {} (or best runs for longer than {} steps)",
                i, results.best_steps, args.max_steps
            ).unwrap();

            for program in results.best_programs {
                writeln!(f, "{}", program).unwrap();
            }

            writeln!(
                f,
                "Unknown programs (did not halt after {} steps)",
                args.max_steps
            )
            .unwrap();

            for program in &results.unknown_programs {
                writeln!(f, "{}", program).unwrap();
            }
            let total = results.num_halted + results.num_looping + results.unknown_programs.len();
            writeln!(
                f,
                "halted/looping/unknown = {} + {} + {} = {}",
                results.num_halted,
                results.num_looping,
                results.unknown_programs.len(),
                total
            )
            .unwrap();
            writeln!(f, "L + ratio: {}/{}", total, results.lexiographic_size).unwrap();
        }
    }
}
