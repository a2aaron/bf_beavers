#[cfg(test)]
mod tests {
    use std::{collections::HashMap, convert::TryFrom};

    use bf_beavers::{
        bf::{self, CompileError, ExecutionContext, ExecutionStatus, Instr, Program},
        generate,
    };

    #[derive(Debug)]
    struct SimpleExecutionContext {
        memory: Vec<u8>,
        memory_pointer: usize,
        program: Vec<bf::Instr>,
        program_pointer: usize,
        loop_dict: HashMap<usize, usize>,
    }

    impl SimpleExecutionContext {
        fn new(program: &bf::Program) -> SimpleExecutionContext {
            let program = program.original_instrs().to_vec();

            let loop_dict = loop_dict(&program).unwrap();

            SimpleExecutionContext {
                memory: vec![0; 256],
                memory_pointer: 0,
                program_pointer: 0,
                program,
                loop_dict,
            }
        }

        fn step(&mut self) -> (SimpleExecutionState, usize) {
            let instruction = self.program.get(self.program_pointer);

            match instruction {
                None => (SimpleExecutionState::Halted, 0),
                Some(instruction) => {
                    match instruction {
                        Instr::Plus => {
                            self.memory[self.memory_pointer] =
                                self.memory[self.memory_pointer].wrapping_add(1)
                        }
                        Instr::Minus => {
                            self.memory[self.memory_pointer] =
                                self.memory[self.memory_pointer].wrapping_sub(1)
                        }
                        Instr::Left => {
                            self.memory_pointer = self.memory_pointer.saturating_sub(1);
                        }
                        Instr::Right => {
                            self.memory_pointer += 1;
                            if self.memory_pointer >= self.memory.len() {
                                self.memory.push(0);
                            }
                        }
                        Instr::StartLoop => {
                            if self.memory[self.memory_pointer] == 0 {
                                let end_loop = self.loop_dict[&self.program_pointer];
                                self.program_pointer = end_loop;
                            }
                        }
                        Instr::EndLoop => {
                            if self.memory[self.memory_pointer] != 0 {
                                let start_loop = self.loop_dict[&self.program_pointer];
                                self.program_pointer = start_loop;
                            }
                        }
                    }
                    self.program_pointer += 1;
                    if self.program.get(self.program_pointer).is_none() {
                        (SimpleExecutionState::Halted, 1)
                    } else {
                        (SimpleExecutionState::Running, 1)
                    }
                }
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum SimpleExecutionState {
        Halted,
        Running,
    }

    fn loop_dict(program: &[Instr]) -> Result<HashMap<usize, usize>, CompileError> {
        use Instr::*;
        let mut hashmap = HashMap::new();
        let mut startloop_locs = Vec::new();
        for (i, &instr) in program.iter().enumerate() {
            match instr {
                Plus | Minus | Left | Right => (),
                StartLoop => {
                    startloop_locs.push(i);
                }
                EndLoop => {
                    match startloop_locs.pop() {
                        Some(start_loop) => {
                            hashmap.insert(i, start_loop);
                            hashmap.insert(start_loop, i);
                        }
                        None => return Err(CompileError::UnmatchedEndLoop { index: i }),
                    };
                }
            }
        }

        if !startloop_locs.is_empty() {
            Err(CompileError::UnmatchedStartLoops {
                indicies: startloop_locs,
            })
        } else {
            Ok(hashmap)
        }
    }

    fn eval(
        program: &Program,
        max_steps: usize,
    ) -> ((ExecutionStatus, usize), (SimpleExecutionState, usize)) {
        let mut real_ctx = ExecutionContext::new(program);
        let mut simple_ctx = SimpleExecutionContext::new(program);

        let mut real_state = ExecutionStatus::Running;
        let mut real_steps = 0;
        for _ in 0..max_steps {
            let (delta, state) = real_ctx.step();
            real_steps += delta;
            real_state = state;
            if real_state != ExecutionStatus::Running {
                break;
            }
        }

        let max_steps = match real_state {
            ExecutionStatus::Halted => real_steps,
            ExecutionStatus::Running => max_steps,
            ExecutionStatus::InfiniteLoop(_) => real_steps * 2,
        };

        let mut simple_state = SimpleExecutionState::Running;
        let mut simple_steps = 0;
        for _ in 0..=max_steps {
            let (state, steps) = simple_ctx.step();
            simple_steps += steps;
            if state == SimpleExecutionState::Halted {
                simple_state = SimpleExecutionState::Halted;
                break;
            }
        }

        ((real_state, real_steps), (simple_state, simple_steps))
    }

    fn assert_model_matches(
        program: &Program,
        max_steps: usize,
    ) -> ((ExecutionStatus, usize), (SimpleExecutionState, usize)) {
        let ((real_state, real_steps), (simple_state, simple_steps)) = eval(program, max_steps);
        match (&real_state, simple_state) {
            (ExecutionStatus::Running, SimpleExecutionState::Running) => (),
            (ExecutionStatus::InfiniteLoop(_), SimpleExecutionState::Running) => (),
            (ExecutionStatus::Halted, SimpleExecutionState::Halted) => {
                assert_eq!(real_steps, simple_steps, "Program: {}", program);
            }
            (real_state, simple_state) => {
                println!(
                    "Mismatch for program {}\n(Real: {:#?}, Simple: {:#?})",
                    program, real_state, simple_state
                );
                panic!();
            }
        }
        ((real_state, real_steps), (simple_state, simple_steps))
    }

    fn assert_halting(program: &Program, max_steps: usize) {
        let ((real_state, real_steps), (simple_state, simple_steps)) = eval(program, max_steps);
        assert_eq!(simple_state, SimpleExecutionState::Halted);
        assert_eq!(real_state, ExecutionStatus::Halted);
        assert_eq!(real_steps, simple_steps);
    }

    #[test]
    fn test_specific_halting() {
        let program = Program::try_from(">").unwrap();
        assert_halting(&program, 10_000);

        let program = Program::try_from(">>>>>>>+[<+]").unwrap();
        assert_halting(&program, 10_000);

        let program = Program::try_from(">>+>>>>>>>>-<<<<<<<<[>+]").unwrap();
        assert_halting(&program, 10_000);

        let program = Program::try_from("++>---[<[-]++>+]").unwrap();
        assert_halting(&program, 10_000);
    }

    #[test]
    fn test_model_checked() {
        for length in 0..8 {
            let mut num_halted = 0;
            let mut num_looping = 0;
            let mut num_unknown = 0;

            for (i, program) in generate::brute_force_iterator(length).enumerate() {
                if i % 10000 == 0 {
                    eprintln!("{}", program);
                }
                let max_steps = 10_000;
                let ((real_state, _), _) = assert_model_matches(&program, max_steps);
                match real_state {
                    ExecutionStatus::Halted => num_halted += 1,
                    ExecutionStatus::InfiniteLoop(_) => num_looping += 1,
                    ExecutionStatus::Running => num_unknown += 1,
                }
            }
            println!(
                "length: {}, halt: {}, loop: {}, unknown: {}",
                length, num_halted, num_looping, num_unknown
            );
        }
    }
}
