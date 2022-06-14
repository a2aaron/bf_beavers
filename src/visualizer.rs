use std::{collections::BTreeMap, io::stdout};

use crossterm::{
    cursor,
    event::{Event, KeyCode, KeyModifiers},
    style::Stylize,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use thousands::Separable;

use crate::bf::{ExecutionContext, ExecutionStatus, Program};

#[derive(Debug, Clone)]
struct HistoryData {
    real_steps: usize,
    status: ExecutionStatus,
    exec_ctx: ExecutionContext,
}

impl HistoryData {
    fn new(program: &Program) -> HistoryData {
        HistoryData {
            real_steps: 0,
            status: ExecutionStatus::Running,
            exec_ctx: ExecutionContext::new(program),
        }
    }

    fn step(&mut self) {
        let (delta, new_status) = self.exec_ctx.step();
        self.real_steps += delta;
        self.status = new_status;
    }
}

struct History {
    history: BTreeMap<usize, HistoryData>,
    program: Program,
}

impl History {
    fn new(program: &Program) -> History {
        History {
            history: BTreeMap::new(),
            program: program.clone(),
        }
    }

    /// Return the HistoryData corresponding to step `step`. This function attempts to cache results when possible.
    fn get(&mut self, step: usize) -> HistoryData {
        if self.history.contains_key(&step) {
            self.history[&step].clone()
        } else {
            // Get the nearest entry below the step count.
            let nearest_lower_entry = self.history.range(..step).next_back();
            let (steps_to_run, mut history_data) = match nearest_lower_entry {
                Some((lower_steps, history_data)) => (step - lower_steps, history_data.clone()),
                None => (step, HistoryData::new(&self.program)),
            };

            // Advance the execution context to the desired step.
            for i in 0..steps_to_run {
                let step = (step - steps_to_run) + i;
                history_data.step();

                // We cache every 1000th step here because it is likely that the user will want to keep going backwards.
                // Caching some intermediate steps avoids having to recompute a lot of work each time.
                if step % 1000 == 0 && !self.history.contains_key(&step) {
                    self.history.insert(step, history_data.clone());
                }
            }

            self.history.insert(step, history_data.clone());
            history_data
        }
    }

    fn total_cells_allocated(&self) -> usize {
        self.history
            .values()
            .map(|history| history.exec_ctx.total_cells_allocated())
            .sum()
    }
}

pub fn run(program: &Program, starting_step: usize) {
    fn print_state(history: &mut History, curr_step: usize) {
        crossterm::execute! { stdout(), cursor::MoveTo(0,0) }.unwrap();
        crossterm::execute! { stdout(), Clear(ClearType::All) }.unwrap();

        let HistoryData {
            status,
            real_steps,
            exec_ctx,
        } = &history.get(curr_step);

        let displayed_status = crossterm::style::style(format!("{:?}", status));
        let displayed_status = match status {
            ExecutionStatus::Running => displayed_status,
            ExecutionStatus::Halted => displayed_status.on_red(),
            ExecutionStatus::InfiniteLoop(_) => displayed_status.on_cyan(),
        };
        println!(
            "Steps: {} (Actual: {}), Status: {}",
            curr_step, real_steps, displayed_status
        );
        println!(
            "Total cells allocated: {}",
            history.total_cells_allocated().separate_with_commas()
        );

        exec_ctx.print_state(true);
    }
    let mut history = History::new(program);
    let mut curr_step = starting_step;

    crossterm::execute! { stdout(), EnterAlternateScreen }.unwrap();
    print_state(&mut history, curr_step);

    'outer: loop {
        crossterm::terminal::enable_raw_mode().unwrap();
        let event = crossterm::event::read().unwrap();
        crossterm::terminal::disable_raw_mode().unwrap();

        if let Event::Key(event) = event {
            // If shift is held, jump to the end/start of this loop.
            let exec_ctx = &history.get(curr_step).exec_ctx;
            let corresponding_loop = if event.modifiers.contains(KeyModifiers::SHIFT) {
                exec_ctx.current_loop_bounds()
            } else {
                None
            };

            loop {
                match event.code {
                    KeyCode::Left | KeyCode::Char('a') => {
                        curr_step = curr_step.saturating_sub(1);
                    }
                    KeyCode::Right | KeyCode::Char('d') => {
                        curr_step += 1;
                    }
                    KeyCode::Esc | KeyCode::Char('q') => break 'outer,
                    _ => (),
                }

                let exec_ctx = &history.get(curr_step).exec_ctx;
                if let Some((start, end)) = corresponding_loop && (start..end).contains(&exec_ctx.program_pointer()) {
                    continue;
                } else {
                     break;
                }
            }
        }
        print_state(&mut history, curr_step);
    }
    stdout().execute(LeaveAlternateScreen).unwrap();
}
