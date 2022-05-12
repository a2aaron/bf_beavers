#[cfg(test)]
mod tests {
    use std::{collections::HashMap, convert::TryFrom};

    use bf_beavers::{
        bf::{self, CompileError, ExecutionContext, ExecutionState, Instr, Program},
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
            let program = program.original_instrs.clone();

            let loop_dict = loop_dict(&program).unwrap();

            SimpleExecutionContext {
                memory: vec![0; 256],
                memory_pointer: 0,
                program_pointer: 0,
                program,
                loop_dict,
            }
        }

        fn step(&mut self) -> SimpleExecutionState {
            let instruction = self.program.get(self.program_pointer);

            match instruction {
                None => SimpleExecutionState::Halted,
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
                    SimpleExecutionState::Running
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

    fn eval(program: &Program, max_steps: usize) -> (Option<ExecutionState>, SimpleExecutionState) {
        let mut real_ctx = ExecutionContext::new(program);
        let mut simple_ctx = SimpleExecutionContext::new(program);

        let mut real_state = None;
        for _ in 1..max_steps {
            let (_, state) = real_ctx.step();
            if state != ExecutionState::Running {
                real_state = Some(state);
                break;
            }
        }

        let mut simple_state = SimpleExecutionState::Running;
        for _ in 1..max_steps {
            let state = simple_ctx.step();
            if state == SimpleExecutionState::Halted {
                simple_state = SimpleExecutionState::Halted;
                break;
            }
        }

        (real_state, simple_state)
    }

    fn assert_model_matches(program: &Program, max_steps: usize) {
        let (real_state, simple_state) = eval(program, max_steps);
        match (real_state, simple_state) {
            (None, SimpleExecutionState::Running) => (),
            (Some(ExecutionState::Halted), SimpleExecutionState::Halted) => (),
            (Some(ExecutionState::InfiniteLoop(_)), SimpleExecutionState::Running) => (),
            (real_state, simple_state) => {
                println!(
                    "Mismatch for program {}\n(Real: {:#?}, Simple: {:#?})",
                    program, real_state, simple_state
                );
                panic!();
            }
        }
    }

    #[test]
    fn test_specific() {
        let program = Program::try_from(">>>>>>>+[<+]").unwrap();
        assert_model_matches(&program, 10_000);

        let program = Program::try_from(">>+>>>>>>>>-<<<<<<<<[>+]").unwrap();
        assert_model_matches(&program, 10_000);
    }

    #[test]
    fn test_model_checked() {
        for (i, program) in generate::brute_force_chain(0..8).enumerate() {
            if i % 1000 == 0 {
                eprintln!("{}", program);
            }
            let max_steps = 10_000;
            assert_model_matches(&program, max_steps)
        }
    }
}
