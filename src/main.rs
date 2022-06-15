#![feature(let_chains)]
#![feature(mixed_integer_ops)]
#![feature(iter_intersperse)]

pub mod visualizer;

use std::{convert::TryFrom, io::Write};

use rayon::prelude::*;

use clap::Parser;

use bf_beavers::{
    bf::{self, ExecutionStatus},
    generate,
};

fn step_count(program: &bf::Program, max_steps: usize) -> (ExecutionStatus, Option<usize>, usize) {
    let mut ctx = bf::ExecutionContext::new(program);
    let mut total_real_steps = 0;
    for _ in 1..max_steps {
        let (real_steps, state) = ctx.step();
        total_real_steps += real_steps;
        match state {
            ExecutionStatus::Halted | ExecutionStatus::InfiniteLoop(_) => {
                return (state, Some(total_real_steps), ctx.tape_length());
            }
            ExecutionStatus::Running => (),
        }
    }
    (ExecutionStatus::Running, None, ctx.tape_length())
}

struct BusyBeaverResults {
    best_programs: Vec<bf::Program>,
    best_steps: usize,
    hardest_to_prove: Option<(usize, bf::Program)>,
    max_tape_length: usize,
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
            if let Some(print_every) = print_every && i % print_every == 0 && *i != 0 {
                eprintln!("{}: {}", i, program)
            }
        })
        .par_bridge()
        .map(|(_, program)| {
            let (state, step, max_tape_length) = step_count(&program, max_steps);
            match state {
                ExecutionStatus::Running => BusyBeaverResults {
                    best_programs: vec![],
                    best_steps: 0,
                    max_tape_length,
                    hardest_to_prove: None,
                    unknown_programs: vec![program],
                    num_halted: 0,
                    num_looping: 0,
                    lexiographic_size,
                },
                ExecutionStatus::Halted => BusyBeaverResults {
                    best_programs: vec![program],
                    best_steps: step.unwrap(),
                    max_tape_length,
                    hardest_to_prove: None,
                    unknown_programs: vec![],
                    num_halted: 1,
                    num_looping: 0,
                    lexiographic_size,
                },
                ExecutionStatus::InfiniteLoop(_) => BusyBeaverResults {
                    best_programs: vec![],
                    best_steps: 0,
                    max_tape_length,
                    hardest_to_prove: Some((step.unwrap(), program)),
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
                max_tape_length: 0,
                hardest_to_prove: None,
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
                hardest_to_prove: match (a.hardest_to_prove, b.hardest_to_prove) {
                    (Some((a_steps, a_prog)), Some((b_steps, b_prog))) => {
                        if a_steps > b_steps {
                            Some((a_steps, a_prog))
                        } else {
                            Some((b_steps, b_prog))
                        }
                    }
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                },
                max_tape_length: a.max_tape_length.max(b.max_tape_length),
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
                let (state, steps, _) = step_count(&program, args.max_steps);
                match state {
                    ExecutionStatus::Running => {
                        println!("Timed out (runs longer than {} steps)", args.max_steps)
                    }
                    ExecutionStatus::Halted => println!("Halts in {} steps", steps.unwrap()),
                    ExecutionStatus::InfiniteLoop(reason) => {
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
            let results = beaver(i, args.max_steps, args.print_every);

            let mut f = std::fs::File::create(format!("length_{}.txt", i)).unwrap();
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
            writeln!(
                f,
                "L + ratio: {}/{} ({:.1}%)",
                total,
                results.lexiographic_size,
                100.0 * total as f32 / results.lexiographic_size as f32
            )
            .unwrap();
            writeln!(f, "max tape length: {}", results.max_tape_length).unwrap();
            if let Some((steps, program)) = results.hardest_to_prove {
                writeln!(
                    f,
                    "hardest to prove: {} ({} steps required)",
                    program, steps,
                )
                .unwrap();
            }
        }
    }
}
