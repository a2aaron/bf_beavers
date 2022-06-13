use std::{
    collections::{hash_map, HashMap},
    convert::TryInto,
    io::stdout,
};

use crossterm::{
    cursor,
    event::{Event, KeyCode, KeyModifiers},
    style::Stylize,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

use crate::bf::{ExecutionContext, ExecutionStatus, Program};

#[derive(Debug, Clone)]
struct HistoryData {
    real_steps: usize,
    status: ExecutionStatus,
    exec_ctx: ExecutionContext,
}

impl HistoryData {
    fn new(real_steps: usize, status: ExecutionStatus, exec_ctx: &ExecutionContext) -> HistoryData {
        HistoryData {
            real_steps,
            status,
            exec_ctx: exec_ctx.clone(),
        }
    }
}

struct History {
    history: HashMap<usize, [HistoryData; 1024]>,
    program: Program,
}

impl History {
    fn new(program: &Program) -> History {
        History {
            history: HashMap::new(),
            program: program.clone(),
        }
    }

    fn get(&mut self, step: usize) -> HistoryData {
        let nearest_gradiation = step / 1024;
        let entry = match self.history.entry(nearest_gradiation) {
            hash_map::Entry::Occupied(entry) => entry,
            hash_map::Entry::Vacant(entry) => {
                let mut real_steps = 0;
                let mut exec_ctx = ExecutionContext::new(&self.program);
                for _ in 0..nearest_gradiation * 1024 {
                    let (real_steps_delta, _) = exec_ctx.step();
                    real_steps += real_steps_delta;
                }
                let mut array = Vec::with_capacity(1024);
                array.push(HistoryData::new(
                    real_steps,
                    ExecutionStatus::Running,
                    &exec_ctx,
                ));
                for _ in 0..1023 {
                    let (real_steps_delta, status) = exec_ctx.step();
                    real_steps += real_steps_delta;
                    array.push(HistoryData::new(real_steps, status, &exec_ctx));
                }

                entry.insert_entry(array.try_into().unwrap())
            }
        };
        entry.get()[step - nearest_gradiation * 1024].clone()
    }
}

pub fn run(program: &Program, starting_step: usize) {
    fn print_state(history: &HistoryData, curr_step: usize) {
        crossterm::execute! { stdout(), cursor::MoveTo(0,0) }.unwrap();
        crossterm::execute! { stdout(), Clear(ClearType::All) }.unwrap();

        let displayed_status = crossterm::style::style(format!("{:?}", history.status));
        let displayed_status = match history.status {
            ExecutionStatus::Running => displayed_status,
            ExecutionStatus::Halted => displayed_status.on_red(),
            ExecutionStatus::InfiniteLoop(_) => displayed_status.on_cyan(),
        };
        println!(
            "Steps: {} (Actual: {}), Status: {}",
            curr_step, history.real_steps, displayed_status
        );

        history.exec_ctx.print_state(true);
    }
    let mut history = History::new(program);
    let mut curr_step = starting_step;

    crossterm::execute! { stdout(), EnterAlternateScreen }.unwrap();
    print_state(&history.get(curr_step), curr_step);

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
        print_state(&history.get(curr_step), curr_step);
    }
    stdout().execute(LeaveAlternateScreen).unwrap();
}
