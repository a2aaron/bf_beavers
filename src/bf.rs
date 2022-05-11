use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt::Display;

const INITAL_MEMORY: usize = 1;
const EXTEND_MEMORY_AMOUNT: usize = 1;

#[derive(Debug)]
pub struct ExecutionContext {
    memory: Vec<u8>,
    memory_pointer: usize,
    program: Program,
    program_pointer: usize,
    active_loop_spans: HashMap<usize, LoopSpan>,
    loop_spans: HashMap<usize, Vec<LoopSpan>>,
}

impl ExecutionContext {
    pub fn new(program: &Program) -> ExecutionContext {
        let mut loop_spans = HashMap::new();
        for (i, &instr) in program.extended_instrs.iter().enumerate() {
            if instr == ExtendedInstr::BaseInstr(Instr::StartLoop) {
                loop_spans.insert(i, vec![]);
            }
        }

        ExecutionContext {
            memory: vec![0; INITAL_MEMORY],
            memory_pointer: 0,
            program_pointer: 0,
            program: program.clone(),
            loop_spans,
            active_loop_spans: HashMap::new(),
        }
    }

    pub fn step(&mut self) -> (usize, ExecutionState) {
        let instruction = self.program.get(self.program_pointer);

        match instruction {
            None => (0, ExecutionState::Halted),
            Some(instruction) => match instruction {
                ExtendedInstr::BaseInstr(instruction) => {
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
                            for loop_span in self.active_loop_spans.values_mut() {
                                loop_span.record_left();
                            }
                        }
                        Instr::Right => {
                            self.memory_pointer += 1;
                            if self.memory_pointer >= self.memory.len() {
                                self.memory.extend([0; EXTEND_MEMORY_AMOUNT].iter());
                            }

                            for loop_span in self.active_loop_spans.values_mut() {
                                loop_span.record_right();
                            }
                        }
                        Instr::StartLoop => {
                            let start_loop = self.program_pointer;
                            if self.memory[self.memory_pointer] == 0 {
                                // Loop not taken. Don't bother with loop spans.
                                let end_loop = self
                                    .program
                                    .matching_loop(start_loop)
                                    .expect("missing StartLoop dict entry!");
                                self.program_pointer = end_loop;
                            } else {
                                // Loop taken. Start recording a loop span.
                                self.start_recording_loop_span(start_loop);
                            }
                        }
                        Instr::EndLoop => {
                            let start_loop = self
                                .program
                                .matching_loop(self.program_pointer)
                                .expect("missing EndLoop dict entry!");
                            // Stop recording the loop
                            self.end_recording_loop_span(start_loop);

                            if self.memory[self.memory_pointer] != 0 {
                                // Loop taken.
                                self.program_pointer = start_loop;

                                // Start a new loop-span recording
                                self.start_recording_loop_span(start_loop);

                                // Check if this span matches any prior union-span from before. If so, then we hit a loop.
                                // If a loop is detected, then signal that a loop has occured.
                                if let Some((prior, current)) = self.check_loop_spans(start_loop) {
                                    self.program_pointer += 1;
                                    return (
                                        1,
                                        ExecutionState::InfiniteLoop(LoopReason::LoopSpan {
                                            prior,
                                            current,
                                        }),
                                    );
                                }
                            } else {
                                // Loop not taken. Reset the loop span history for this loop.
                                self.reset_loop_spans(start_loop);
                            }
                        }
                    }
                    self.program_pointer += 1;
                    (1, ExecutionState::Running)
                }
                ExtendedInstr::LoopIfNonzero => {
                    if self.memory[self.memory_pointer] == 0 {
                        self.program_pointer += 1;
                        (2, ExecutionState::Running)
                    } else {
                        (2, ExecutionState::InfiniteLoop(LoopReason::LoopIfNonzero))
                    }
                }
            },
        }
    }

    fn start_recording_loop_span(&mut self, loop_index: usize) {
        assert!(!self.active_loop_spans.contains_key(&loop_index));
        let loop_span = LoopSpan::new(self.memory.clone(), self.memory_pointer);

        let old_value = self.active_loop_spans.insert(loop_index, loop_span);
        assert!(old_value.is_none());
    }

    fn end_recording_loop_span(&mut self, loop_index: usize) {
        assert!(self.active_loop_spans.contains_key(&loop_index));

        let loop_span = self.active_loop_spans.remove(&loop_index).unwrap();

        self.loop_spans
            .get_mut(&loop_index)
            .unwrap()
            .push(loop_span);
    }

    fn reset_loop_spans(&mut self, loop_index: usize) {
        self.loop_spans.get_mut(&loop_index).unwrap().clear()
    }

    fn check_loop_spans(&self, loop_index: usize) -> Option<(LoopSpan, LoopSpan)> {
        let loop_spans = &self.loop_spans[&loop_index];

        // TODO: Add unioning of prior spans. This currently only detects 1-periodic loops.
        if loop_spans.len() >= 2 {
            let current = &loop_spans[loop_spans.len() - 1];
            let prior = &loop_spans[loop_spans.len() - 2];
            if LoopSpan::equals(prior, current) {
                Some((prior.clone(), current.clone()))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn print_state(&self, show_execution_history: bool) {
        let memory = array_to_string(&self.memory);
        let memory_pointer = highlight(self.memory_pointer);

        let program = self
            .program
            .extended_instrs
            .iter()
            .map(|instr| format!("{}", instr))
            .collect::<String>();

        let program_ptr = self
            .program
            .extended_instrs
            .iter()
            .enumerate()
            .map(|(i, _)| {
                if i == self.program_pointer {
                    "^".to_string()
                } else {
                    " ".to_string()
                }
            })
            .collect::<String>();

        println!("Memory: {}", memory);
        println!("        {}", memory_pointer);
        println!("Program: {}", program);
        println!("         {}", program_ptr);

        if show_execution_history {
            for (idx, states) in self.loop_spans.iter() {
                if states.len() > 10 {
                    println!(
                        "history for instr @ {} (too long: {} entries)",
                        idx,
                        states.len()
                    );
                } else {
                    println!("history for instr @ {}", idx);
                    for state in states {
                        println!("{}", state);
                    }
                }
            }
            for (idx, state) in self.active_loop_spans.iter() {
                println!("active span for instr @ {}:\n{}", idx, state);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopSpan {
    // A snapshot of memory at the start of the loop
    memory_at_loop_start: Vec<u8>,
    // An index into the program memory denoting the position of the memory pointer at the start of the loop.
    starting_memory_pointer: usize,
    // An index into the program memory denoting the position of the memory pointer at the current point in the loop.
    current_memory_pointer: usize,
    // The currently lowest index the memory pointer touched during the loop
    min_index: usize,
    // The currently highest index the memory pointer touched during the loop
    max_index: usize,
}

impl LoopSpan {
    fn new(memory: Vec<u8>, starting_position: usize) -> LoopSpan {
        LoopSpan {
            memory_at_loop_start: memory,
            starting_memory_pointer: starting_position,
            current_memory_pointer: starting_position,
            min_index: starting_position,
            max_index: starting_position,
        }
    }

    fn record_left(&mut self) {
        self.current_memory_pointer = self.current_memory_pointer.saturating_sub(1);
        if self.current_memory_pointer < self.min_index {
            self.min_index = self.current_memory_pointer;
        }
    }

    fn record_right(&mut self) {
        self.current_memory_pointer += 1;
        if self.current_memory_pointer > self.max_index {
            self.max_index = self.current_memory_pointer;
        }
    }

    // Return the slice of memory that is considered part of the loop span.
    fn memory_mask(&self) -> &[u8] {
        // Remove trailing zeros from memory snap shot
        let first_nonzero = {
            let mut first_nonzero = self.memory_at_loop_start.len() - 1;
            for i in (0..self.memory_at_loop_start.len()).rev() {
                if self.memory_at_loop_start[i] != 0 {
                    first_nonzero = i;
                    break;
                }
            }
            first_nonzero
        };

        let min_index = self.min_index.min(first_nonzero);
        let max_index = self.max_index.min(first_nonzero);

        // Now check the displacement. If the displacement is negative, then
        // consider everything to the left of the span to be included. Otherwise
        // include everything to the right of the span. If the displacement is
        // zero, then don't include anything extra and just return the span as is.
        match self.displacement().cmp(&0) {
            std::cmp::Ordering::Less => &self.memory_at_loop_start[0..=max_index],
            std::cmp::Ordering::Equal => &self.memory_at_loop_start[min_index..],
            std::cmp::Ordering::Greater => &self.memory_at_loop_start[min_index..=max_index],
        }
    }

    fn displacement(&self) -> isize {
        self.current_memory_pointer as isize - self.starting_memory_pointer as isize
    }

    fn equals(a: &LoopSpan, b: &LoopSpan) -> bool {
        let displacements_match = a.displacement() == b.displacement();
        let masks_match = a.memory_mask() == b.memory_mask();

        displacements_match && masks_match
    }
}

impl Display for LoopSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "start: {} curr: {} min: {} max: {}",
            self.starting_memory_pointer,
            self.current_memory_pointer,
            self.min_index,
            self.max_index
        )?;
        writeln!(f, "{}", array_to_string(&self.memory_at_loop_start))?;
        writeln!(f, "{}", highlight_range(self.min_index, self.max_index))?;
        Ok(())
    }
}

fn array_to_string(array: &[u8]) -> String {
    array
        .iter()
        .map(|x| format!("{:0>2X}", x))
        .intersperse(" ".to_string())
        .collect()
}

fn highlight(index: usize) -> String {
    (0..=index)
        .map(|i| if index == i { "^^" } else { "  " })
        .intersperse(" ")
        .collect()
}

fn highlight_range(lower: usize, upper: usize) -> String {
    assert!(lower <= upper);
    (0..=upper)
        .map(|index| {
            if lower == index || index == upper {
                "^^"
            } else if lower < index || index < upper {
                "--"
            } else {
                "  "
            }
        })
        .intersperse(" ")
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionState {
    Running,
    Halted,
    InfiniteLoop(LoopReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopReason {
    LoopIfNonzero,
    LoopSpan { prior: LoopSpan, current: LoopSpan },
}

impl Display for LoopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopReason::LoopIfNonzero => write!(f, "LoopIfNonzero instruction triggered"),
            LoopReason::LoopSpan { prior, current } => write!(
                f,
                "LoopSpan triggered. prior span: {} current span: {}",
                prior, current
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub original_instrs: Vec<Instr>,
    extended_instrs: Vec<ExtendedInstr>,
    loop_dict: HashMap<usize, usize>,
}

impl Program {
    pub fn new(instrs: &[Instr]) -> Result<Program, CompileError> {
        let extended_instrs = ExtendedInstr::new(instrs);
        let loop_dict = loop_dict(&extended_instrs)?;
        Ok(Program {
            original_instrs: instrs.to_vec(),
            extended_instrs,
            loop_dict,
        })
    }

    fn get(&self, i: usize) -> Option<ExtendedInstr> {
        self.extended_instrs.get(i).cloned()
    }

    fn matching_loop(&self, i: usize) -> Option<usize> {
        self.loop_dict.get(&i).copied()
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", to_string(&self.original_instrs),)
    }
}

impl TryFrom<&str> for Program {
    type Error = CompileError;

    fn try_from(string: &str) -> Result<Self, Self::Error> {
        let instrs = string
            .chars()
            .filter_map(|x| x.try_into().ok())
            .collect::<Vec<Instr>>();
        Program::new(&instrs)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ExtendedInstr {
    BaseInstr(Instr),
    LoopIfNonzero,
}

impl ExtendedInstr {
    fn new(program: &[Instr]) -> Vec<ExtendedInstr> {
        let mut extended_instrs = vec![];
        let mut i = 0;
        while i < program.len() {
            let this_instr = program[i];
            let next_instr = program.get(i + 1);
            let extended_instr = match (this_instr, next_instr) {
                (Instr::StartLoop, Some(Instr::EndLoop)) => {
                    i += 2;
                    ExtendedInstr::LoopIfNonzero
                }
                (instr, _) => {
                    i += 1;
                    ExtendedInstr::BaseInstr(instr)
                }
            };
            extended_instrs.push(extended_instr);
        }
        extended_instrs
    }
}

impl Display for ExtendedInstr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let chara = match self {
            ExtendedInstr::BaseInstr(Instr::Plus) => '+',
            ExtendedInstr::BaseInstr(Instr::Minus) => '-',
            ExtendedInstr::BaseInstr(Instr::Left) => '<',
            ExtendedInstr::BaseInstr(Instr::Right) => '>',
            ExtendedInstr::BaseInstr(Instr::StartLoop) => '[',
            ExtendedInstr::BaseInstr(Instr::EndLoop) => ']',
            ExtendedInstr::LoopIfNonzero => 'L',
        };
        write!(f, "{}", chara)
    }
}

fn loop_dict(program: &[ExtendedInstr]) -> Result<HashMap<usize, usize>, CompileError> {
    use Instr::*;
    let mut hashmap = HashMap::new();
    let mut startloop_locs = Vec::new();
    for (i, &instr) in program.iter().enumerate() {
        match instr {
            ExtendedInstr::LoopIfNonzero => (),
            ExtendedInstr::BaseInstr(instr) => match instr {
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
            },
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

pub fn to_string(program: &[Instr]) -> String {
    let mut string = String::new();
    for &bf_char in program {
        use Instr::*;
        let letter: char = match bf_char {
            Plus => '+',
            Minus => '-',
            Left => '<',
            Right => '>',
            StartLoop => '[',
            EndLoop => ']',
        };
        string.push(letter);
    }

    string
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Instr {
    Plus,
    Minus,
    Left,
    Right,
    StartLoop,
    EndLoop,
}

impl TryFrom<char> for Instr {
    type Error = ();

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            '+' => Ok(Instr::Plus),
            '-' => Ok(Instr::Minus),
            '<' => Ok(Instr::Left),
            '>' => Ok(Instr::Right),
            '[' => Ok(Instr::StartLoop),
            ']' => Ok(Instr::EndLoop),
            _ => Err(()),
        }
    }
}

impl Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Instr::*;
        let char = match self {
            Plus => '+',
            Minus => '-',
            Left => '<',
            Right => '>',
            StartLoop => '[',
            EndLoop => ']',
        };
        write!(f, "{}", char)
    }
}

#[derive(Debug, Clone)]
pub enum CompileError {
    UnmatchedEndLoop { index: usize },
    UnmatchedStartLoops { indicies: Vec<usize> },
}

impl Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::UnmatchedEndLoop { index } => {
                write!(f, "Unmatched end loop at {}", index)
            }
            CompileError::UnmatchedStartLoops { indicies } => {
                write!(f, "One or more unmatched start loops at {:?}", indicies)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(program: &Program, max_steps: usize) -> Option<ExecutionState> {
        let mut ctx = ExecutionContext::new(program);
        for _ in 1..max_steps {
            let (_, state) = ctx.step();
            if state != ExecutionState::Running {
                return Some(state);
            }
        }
        None
    }

    fn assert_halting(program: &str) {
        let program = Program::try_from(program).unwrap();
        assert_eq!(eval(&program, 9_999_999).unwrap(), ExecutionState::Halted);
    }

    fn assert_not_halting_loop_if_nonzero(program: &str) {
        let program = Program::try_from(program).unwrap();
        let result = matches!(
            eval(&program, 9_999_999).unwrap(),
            ExecutionState::InfiniteLoop(LoopReason::LoopIfNonzero)
        );
        assert!(result);
    }

    fn assert_not_halting_loop_span(program: &str) {
        let program = Program::try_from(program).unwrap();
        let result = matches!(
            eval(&program, 9_999_999).unwrap(),
            ExecutionState::InfiniteLoop(LoopReason::LoopSpan { .. })
        );
        assert!(result);
    }

    #[test]
    fn test_halting() {
        assert_halting("+[-]");
        assert_halting(">+[>++++[-<]>>]");
        assert_halting("+[->++++++[-<]>]");
        assert_halting(">+[>++>+++[-<]>>]");
        assert_halting(">+[>++>+++[-<]>>]+");
        assert_halting("++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++[>+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++<-]>");
    }

    #[test]
    fn test_non_halting_loop_if_nonzero() {
        assert_not_halting_loop_if_nonzero("+[]");
        assert_not_halting_loop_if_nonzero("+<[]");
        assert_not_halting_loop_if_nonzero("-[]");
        assert_not_halting_loop_if_nonzero("-[-[+]+[]]");
        assert_not_halting_loop_if_nonzero("+[[[]]]");
    }

    #[test]
    fn test_non_halting_loop_span() {
        assert_not_halting_loop_span("+[<]");
        assert_not_halting_loop_span("+[-+]");
        assert_not_halting_loop_span("+[[+]-]");
    }
}
