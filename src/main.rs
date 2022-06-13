use std::{convert::TryFrom, io::Write};

use clap::Parser;

use bf_beavers::{
    bf::{self, ExecutionState, LoopReason},
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

fn beaver(length: usize, max_steps: usize, verbose: Option<usize>) -> BusyBeaverResults {
    let mut best_programs = vec![];
    let mut best_steps = 0;

    let mut unknown_programs = vec![];

    let programs = generate::brute_force_iterator(length);

    let mut num_halted = 0;
    let mut num_looping = 0;

    for (i, program) in programs.enumerate() {
        let (state, step_count) = step_count(&program, max_steps);

        if verbose.map(|x| i % x == 0).unwrap_or(false) {
            let prefix = match state {
                ExecutionState::Running => "TIME ",
                ExecutionState::Halted => "HALT ",
                ExecutionState::InfiniteLoop(LoopReason::LoopIfNonzero) => "LOOP1",
                ExecutionState::InfiniteLoop(LoopReason::LoopSpan { .. }) => "LOOP2",
            };
            eprintln!("{}: {}", prefix, program);
        }

        match state {
            ExecutionState::Running => unknown_programs.push(program.clone()),
            ExecutionState::Halted => {
                num_halted += 1;
                let steps = step_count.unwrap();
                if steps > best_steps {
                    best_programs = vec![program];
                    best_steps = steps;
                }
            }
            ExecutionState::InfiniteLoop(_) => num_looping += 1,
        }
    }

    BusyBeaverResults {
        best_programs,
        best_steps,
        unknown_programs,
        num_halted,
        num_looping,
        lexiographic_size: generate::lexiographic_order(length).count(),
    }
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
