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
    cells_allocated: usize,
}

impl History {
    fn new(program: &Program) -> History {
        History {
            history: BTreeMap::new(),
            program: program.clone(),
            cells_allocated: 0,
        }
    }

    /// Return the HistoryData corresponding to step `step`. This function attempts to cache results when possible.
    fn get(&mut self, step: usize) -> HistoryData {
        if self.history.contains_key(&step) {
            self.history[&step].clone()
        } else {
            // Get the nearest entry below the step count.
            let nearest_lower_entry = self.history.range(..step).next_back();
            let (steps_to_run, mut data) = match nearest_lower_entry {
                Some((lower_steps, history_data)) => (step - lower_steps, history_data.clone()),
                None => (step, HistoryData::new(&self.program)),
            };

            // Advance the execution context to the desired step.
            for i in 0..steps_to_run {
                let step = (step - steps_to_run) + i;
                data.step();

                // We cache every 1000th step here because it is likely that the user will want to keep going backwards.
                // Caching some intermediate steps avoids having to recompute a lot of work each time.
                if step % 1000 == 0 && !self.history.contains_key(&step) {
                    self.insert_step(step, data.clone());
                }
            }

            self.insert_step(step, data.clone());
            data
        }
    }

    /// Return the HistoryData corresponding to the soonest time when execution reaches the end
    /// the current loop. This does not cache intermediate steps of the loop. This function
    /// also bails out after 10000 steps.
    fn get_after_this_loop(&mut self, mut step: usize, step_size: isize) -> (HistoryData, usize) {
        let mut data = self.get(step);
        let corresponding_loop = data.exec_ctx.current_loop_bounds();
        match corresponding_loop {
            Some((start, end)) => {
                for _ in 0..10_000 {
                    data.step();
                    step = step.saturating_add_signed(step_size);

                    if step % 1000 == 0 && !self.history.contains_key(&step) {
                        self.insert_step(step, data.clone());
                    }

                    let inside_loop = (start..end).contains(&data.exec_ctx.program_pointer());
                    if !inside_loop || (step == 0 && step_size < 0) {
                        break;
                    }
                }
            }
            None => (),
        }
        (data, step)
    }

    fn insert_step(&mut self, step: usize, data: HistoryData) {
        assert!(!self.history.contains_key(&step));
        self.cells_allocated += data.exec_ctx.total_cells_allocated();
        self.history.insert(step, data);
    }

    fn total_cells_allocated(&self) -> usize {
        self.cells_allocated
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
            "Total cells allocated: {} (in {} cached steps)",
            history.total_cells_allocated().separate_with_commas(),
            history.history.len()
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
            let shift_held = event.modifiers.contains(KeyModifiers::SHIFT);
            match event.code {
                KeyCode::Left | KeyCode::Char('a') => {
                    if shift_held {
                        curr_step = history.get_after_this_loop(curr_step, -1).1;
                    } else {
                        curr_step = curr_step.saturating_sub(1);
                    }
                }
                KeyCode::Right | KeyCode::Char('d') => {
                    if shift_held {
                        curr_step = history.get_after_this_loop(curr_step, 1).1;
                    } else {
                        curr_step += 1;
                    }
                }
                KeyCode::Esc | KeyCode::Char('q') => break 'outer,
                _ => (),
            }
        }
        print_state(&mut history, curr_step);
    }
    stdout().execute(LeaveAlternateScreen).unwrap();
}
