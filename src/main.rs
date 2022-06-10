use std::{convert::TryFrom, io::stdout};

use clap::Parser;
use crossterm::{
    cursor,
    event::{Event, KeyCode},
    style::Stylize,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

use bf_beavers::{
    bf::{self, ExecutionState, LoopReason},
    generate,
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
                return (state, None);
            }
            ExecutionState::Running => (),
        }
    }
    (ExecutionState::Running, None)
}

fn beaver(length: usize, max_steps: usize, verbose: Option<usize>) -> (Vec<bf::Program>, usize) {
    let mut best_programs = vec![];
    let mut best_steps = 0;
    let programs = generate::brute_force_iterator(length);

    let mut num_halted = 0;
    let mut num_looping = 0;
    let mut num_unknown = 0;

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
            ExecutionState::Running => num_unknown += 1,
            ExecutionState::Halted => num_halted += 1,
            ExecutionState::InfiniteLoop(_) => num_looping += 1,
        }

        match step_count {
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

    let total = num_halted + num_looping + num_unknown;
    println!(
        "halted/looping/unknown: {} + {} + {} = {}",
        num_halted, num_looping, num_unknown, total
    );
    println!(
        "generation ratio = {}/{}",
        total,
        generate::lexiographic_order(length).count(),
    );
    (best_programs, best_steps)
}

fn visualizer(program: bf::Program) {
    let mut curr_exec = bf::ExecutionContext::new(&program);

    let mut history = vec![((0, ExecutionState::Running), curr_exec.clone())];

    let mut curr_step = 0_usize;

    let (mut cols, _) = crossterm::terminal::size().unwrap();
    crossterm::execute! { stdout(), EnterAlternateScreen }.unwrap();

    loop {
        match crossterm::event::read().unwrap() {
            Event::Key(event) => match event.code {
                KeyCode::Left | KeyCode::Char('a') => curr_step = curr_step.saturating_sub(1),
                KeyCode::Right | KeyCode::Char('d') => curr_step += 1,
                KeyCode::Esc | KeyCode::Char('q') => break,
                _ => (),
            },
            Event::Resize(new_cols, _) => cols = new_cols,
            _ => (),
        }

        while curr_step >= history.len() {
            let step_result = curr_exec.step();
            history.push((step_result, curr_exec.clone()));
        }

        let ((_, state), exe_ctx) = &history[curr_step];

        crossterm::execute! { stdout(), cursor::MoveTo(0,0) }.unwrap();
        crossterm::execute! { stdout(), Clear(ClearType::All) }.unwrap();

        let displayed_state = crossterm::style::style(format!("{:?}", state));
        let displayed_state = match state {
            ExecutionState::Running => displayed_state,
            ExecutionState::Halted => displayed_state.on_red(),
            ExecutionState::InfiniteLoop(_) => displayed_state.on_cyan(),
        };
        println!(
            "Steps: {}, State: {}, cols: {}",
            curr_step, displayed_state, cols
        );

        exe_ctx.print_state(true);
    }
    stdout().execute(LeaveAlternateScreen).unwrap();
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Interactive mode, with a BF program to visualize
    #[clap(short, long, name = "BF PROGRAM")]
    interactive: Option<String>,
    #[clap(long, default_value_t = 50_000)]
    max_steps: usize,
    #[clap(long, default_value_t = 8)]
    max_length: usize,
    #[clap(short, long)]
    print_every: Option<usize>,
}
fn main() {
    let args = Args::parse();
    if let Some(program) = args.interactive {
        match bf::Program::try_from(program.as_str()) {
            Ok(program) => {
                println!("Visualizing {}", program);
                visualizer(program);
                println!("Exiting...");
            }
            Err(err) => println!("Cannot compile {} (reason: {})", program, err),
        }
    } else {
        for i in 0..=args.max_length {
            println!("---");
            let (programs, steps) = beaver(i, args.max_steps, args.print_every);

            println!(
            "Best Programs for Beaver (length = {}, steps = {} or best runs for longer than {})",
            i, steps, args.max_steps
        );
            for program in programs {
                println!("{}", program);
            }
        }
    }
}
