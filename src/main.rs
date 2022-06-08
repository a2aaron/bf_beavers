use std::{convert::TryFrom, io::stdout};

use clap::Parser;
use crossterm::{
    cursor,
    event::{Event, KeyCode},
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, ScrollUp},
    ExecutableCommand,
};

use bf_beavers::{
    bf::{self, ExecutionState, LoopReason},
    generate,
};

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

        let ((_, displayed_state), displayed_exec) = &history[curr_step];

        crossterm::execute! { stdout(), cursor::MoveTo(0,0) }.unwrap();
        crossterm::execute! { stdout(), Clear(ClearType::All) }.unwrap();
        println!(
            "Steps: {}, State: {:?}, cols: {}",
            curr_step, displayed_state, cols
        );
        displayed_exec.print_state(true);
    }
    stdout().execute(LeaveAlternateScreen).unwrap();
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Interactive mode, with a BF program to visualize
    #[clap(short, long)]
    interactive: Option<String>,
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
        let max_steps = 50_000;
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
