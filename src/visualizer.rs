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

struct History {
    history: HashMap<usize, [((usize, ExecutionStatus), ExecutionContext); 1024]>,
    program: Program,
}

impl History {
    fn new(program: &Program) -> History {
        History {
            history: HashMap::new(),
            program: program.clone(),
        }
    }

    fn get(&mut self, step: usize) -> ((usize, ExecutionStatus), ExecutionContext) {
        let nearest_gradiation = step / 1024;
        let entry = match self.history.entry(nearest_gradiation) {
            hash_map::Entry::Occupied(entry) => entry,
            hash_map::Entry::Vacant(entry) => {
                let mut exec_ctx = ExecutionContext::new(&self.program);
                for _ in 0..nearest_gradiation * 1024 {
                    exec_ctx.step();
                }
                let mut array = Vec::with_capacity(1024);
                array.push(((1, ExecutionStatus::Running), exec_ctx.clone()));
                for _ in 0..1023 {
                    let state = exec_ctx.step();
                    array.push((state, exec_ctx.clone()))
                }

                entry.insert_entry(array.try_into().unwrap())
            }
        };
        entry.get()[step - nearest_gradiation * 1024].clone()
    }
}

pub fn run(program: &Program, starting_step: usize) {
    fn print_state(
        ((_, state), exe_ctx): &((usize, ExecutionStatus), ExecutionContext),
        curr_step: usize,
    ) {
        crossterm::execute! { stdout(), cursor::MoveTo(0,0) }.unwrap();
        crossterm::execute! { stdout(), Clear(ClearType::All) }.unwrap();

        let displayed_state = crossterm::style::style(format!("{:?}", state));
        let displayed_state = match state {
            ExecutionStatus::Running => displayed_state,
            ExecutionStatus::Halted => displayed_state.on_red(),
            ExecutionStatus::InfiniteLoop(_) => displayed_state.on_cyan(),
        };
        println!("Steps: {}, State: {}", curr_step, displayed_state);

        exe_ctx.print_state(true);
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
            let curr_exec = &history.get(curr_step).1;
            let corresponding_loop = if event.modifiers.contains(KeyModifiers::SHIFT) {
                curr_exec.current_loop_bounds()
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

                let curr_exec = &history.get(curr_step).1;
                if let Some((start, end)) = corresponding_loop && (start..end).contains(&curr_exec.program_pointer()) {
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
