use std::io::stdout;

use crossterm::{
    cursor,
    event::{Event, KeyCode, KeyModifiers},
    style::Stylize,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

use crate::bf::{ExecutionContext, ExecutionState, Program};

struct History {
    history: Vec<((usize, ExecutionState), ExecutionContext)>,
    latest_exec: ExecutionContext,
}

impl History {
    fn new(program: &Program) -> History {
        let latest_exec = ExecutionContext::new(program);
        History {
            history: vec![((0, ExecutionState::Running), latest_exec.clone())],
            latest_exec,
        }
    }

    fn get(&mut self, step: usize) -> &((usize, ExecutionState), ExecutionContext) {
        while step >= self.history.len() {
            let step_result = self.latest_exec.step();
            self.history.push((step_result, self.latest_exec.clone()));
        }

        &self.history[step]
    }
}

pub fn run(program: &Program, starting_step: usize) {
    fn print_state(
        ((_, state), exe_ctx): &((usize, ExecutionState), ExecutionContext),
        curr_step: usize,
    ) {
        crossterm::execute! { stdout(), cursor::MoveTo(0,0) }.unwrap();
        crossterm::execute! { stdout(), Clear(ClearType::All) }.unwrap();

        let displayed_state = crossterm::style::style(format!("{:?}", state));
        let displayed_state = match state {
            ExecutionState::Running => displayed_state,
            ExecutionState::Halted => displayed_state.on_red(),
            ExecutionState::InfiniteLoop(_) => displayed_state.on_cyan(),
        };
        println!("Steps: {}, State: {}", curr_step, displayed_state);

        exe_ctx.print_state(true);
    }
    let mut history = History::new(program);
    let mut curr_step = starting_step;

    crossterm::execute! { stdout(), EnterAlternateScreen }.unwrap();
    print_state(history.get(curr_step), curr_step);

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
                if let Some((start, end)) = corresponding_loop && start <= curr_exec.program_pointer() && curr_exec.program_pointer() < end {
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
