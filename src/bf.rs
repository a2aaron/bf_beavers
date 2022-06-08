use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt::Display;

use owo_colors::{AnsiColors, OwoColorize};

const INITAL_MEMORY: usize = 1;
const EXTEND_MEMORY_AMOUNT: usize = 1;

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    memory: Vec<u8>,
    memory_pointer: usize,
    program: Program,
    program_pointer: usize,
    loop_span_history: LoopSpanHistory,
}

impl ExecutionContext {
    pub fn new(program: &Program) -> ExecutionContext {
        ExecutionContext {
            memory: vec![0; INITAL_MEMORY],
            memory_pointer: 0,
            program_pointer: 0,
            program: program.clone(),
            loop_span_history: LoopSpanHistory::new(program),
        }
    }

    pub fn step(&mut self) -> (usize, ExecutionState) {
        let instruction = self.program.get(self.program_pointer);

        match instruction {
            None => (0, ExecutionState::Halted),
            Some(instruction) => match instruction {
                ExtendedInstr::BaseInstr(instruction) => {
                    // First, update the loop-spans and figure out if this iteration
                    // is definitely looping.
                    let execution_result = match instruction {
                        Instr::Left => {
                            self.loop_span_history.record_left();
                            (1, ExecutionState::Running)
                        }
                        Instr::Right => {
                            self.loop_span_history.record_right();
                            (1, ExecutionState::Running)
                        }
                        // StartLoop taken. Start recording a loop span.
                        Instr::StartLoop if self.memory[self.memory_pointer] != 0 => {
                            let start_loop = self.program_pointer;
                            self.loop_span_history.start_recording_loop_span(
                                self.memory.clone(),
                                self.memory_pointer,
                                start_loop,
                            );
                            (1, ExecutionState::Running)
                        }
                        // StartLoop not taken. (Ignored, nothing special happens for this)
                        Instr::StartLoop => (1, ExecutionState::Running),
                        // EndLoop taken, stop the old loop-span recording and start a new one
                        Instr::EndLoop if self.memory[self.memory_pointer] != 0 => {
                            let start_loop = self
                                .program
                                .matching_loop(self.program_pointer)
                                .expect("missing EndLoop dict entry!");

                            let check_span_result =
                                self.loop_span_history.end_recording_loop_span(start_loop);
                            self.loop_span_history.start_recording_loop_span(
                                self.memory.clone(),
                                self.memory_pointer,
                                start_loop,
                            );

                            // Check if this span matches any prior union-span from before. If so, then we hit a loop.
                            // If a loop is detected, then signal that a loop has occured.
                            if let Some((prior, current)) = check_span_result {
                                let inf_loop = ExecutionState::InfiniteLoop(LoopReason::LoopSpan {
                                    prior,
                                    current,
                                });
                                (1, inf_loop)
                            } else {
                                (1, ExecutionState::Running)
                            }
                        }
                        // EndLoop not taken. Stop the old loop-span recording and reset the loop span history for this loop history.
                        Instr::EndLoop => {
                            let start_loop = self
                                .program
                                .matching_loop(self.program_pointer)
                                .expect("missing EndLoop dict entry!");

                            self.loop_span_history.end_recording_loop_span(start_loop);
                            self.loop_span_history.reset_past_loop_spans(start_loop);
                            (1, ExecutionState::Running)
                        }
                        _ => (1, ExecutionState::Running),
                    };

                    // Now actually execute the instruction
                    match instruction {
                        Instr::Plus => {
                            self.memory[self.memory_pointer] =
                                self.memory[self.memory_pointer].wrapping_add(1);
                        }
                        Instr::Minus => {
                            self.memory[self.memory_pointer] =
                                self.memory[self.memory_pointer].wrapping_sub(1);
                        }
                        Instr::Left => {
                            self.memory_pointer = self.memory_pointer.saturating_sub(1);
                        }
                        Instr::Right => {
                            self.memory_pointer += 1;
                            if self.memory_pointer >= self.memory.len() {
                                self.memory.extend([0; EXTEND_MEMORY_AMOUNT].iter());
                            }
                        }
                        // StartLoop not taken -- Jump past corresponding EndLoop
                        Instr::StartLoop if self.memory[self.memory_pointer] == 0 => {
                            let start_loop = self.program_pointer;
                            let end_loop = self
                                .program
                                .matching_loop(start_loop)
                                .expect("missing StartLoop dict entry!");
                            self.program_pointer = end_loop;
                        }
                        // EndLoop taken -- Jump past corresponding StartLoop
                        Instr::EndLoop if self.memory[self.memory_pointer] != 0 => {
                            let start_loop = self
                                .program
                                .matching_loop(self.program_pointer)
                                .expect("missing EndLoop dict entry!");
                            self.program_pointer = start_loop;
                        }
                        _ => (),
                    }
                    self.program_pointer += 1;
                    execution_result
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
            for (idx, states) in self.loop_span_history.single_loop_spans.iter() {
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
            for (idx, state) in self.loop_span_history.active_loop_spans.iter() {
                println!("active span for instr @ {}:\n{}", idx, state);
            }
        }
    }
}

//
#[derive(Debug, Clone)]
struct LoopSpanHistory {
    // The list of actively recorded loop spans. A loop which execution is
    // currently inside of has a corresponding active loop span. When the loop
    // finishes (and is re-taken), the loop span is added to the corresponding
    // single_loop_span list.
    active_loop_spans: HashMap<usize, LoopSpan>,
    // List of past recordered loop spans. A given loop span list is cleared
    // any time execution leaves the loop that the loop span list is associated
    // with.
    single_loop_spans: HashMap<usize, Vec<LoopSpan>>,
}

impl LoopSpanHistory {
    fn new(program: &Program) -> LoopSpanHistory {
        let mut past_loop_spans = HashMap::new();
        for (i, &instr) in program.extended_instrs.iter().enumerate() {
            if instr == ExtendedInstr::BaseInstr(Instr::StartLoop) {
                past_loop_spans.insert(i, vec![]);
            }
        }

        let active_loop_spans = HashMap::new();

        LoopSpanHistory {
            active_loop_spans,
            single_loop_spans: past_loop_spans,
        }
    }

    fn record_left(&mut self) {
        for loop_span in self.active_loop_spans.values_mut() {
            loop_span.record_left();
        }
    }

    fn record_right(&mut self) {
        for loop_span in self.active_loop_spans.values_mut() {
            loop_span.record_right();
        }
    }

    // Start recording a new loop span. There must not be another active loop span
    // recording or else this function will panic.
    fn start_recording_loop_span(
        &mut self,
        memory: Vec<u8>,
        starting_position: usize,
        loop_index: usize,
    ) {
        assert!(
            !self.active_loop_spans.contains_key(&loop_index),
            "Recording already exists at index = {} (all spans: {:#?})",
            loop_index,
            self.active_loop_spans
        );
        let loop_span = LoopSpan::new(memory, starting_position);

        let old_value = self.active_loop_spans.insert(loop_index, loop_span);
        assert!(old_value.is_none());
    }

    // End the active loop span recording associated with the given loop index
    // and adds the recording to the loop_index's history.
    // A prior loop span recording must have been started at the same loop index
    // or else this function will panic. Returns Some if the recorded loop span
    // matches a previously recorded loop span.
    fn end_recording_loop_span(&mut self, loop_index: usize) -> Option<(LoopSpan, LoopSpan)> {
        fn check_loop_spans(
            prior_spans: &[LoopSpan],
            current_span: &LoopSpan,
        ) -> Option<(LoopSpan, LoopSpan)> {
            // TODO: Add unioning of prior spans. This currently only detects 1-periodic loops.
            if !prior_spans.is_empty() {
                let prior_span = &prior_spans[prior_spans.len() - 1];
                if prior_span == current_span {
                    Some((prior_span.clone(), current_span.clone()))
                } else {
                    None
                }
            } else {
                None
            }
        }
        assert!(self.active_loop_spans.contains_key(&loop_index));

        let loop_span = self.active_loop_spans.remove(&loop_index).unwrap();

        let loop_span_check = check_loop_spans(&self.single_loop_spans[&loop_index], &loop_span);

        self.single_loop_spans
            .get_mut(&loop_index)
            .unwrap()
            .push(loop_span);

        loop_span_check
    }

    fn reset_past_loop_spans(&mut self, loop_index: usize) {
        self.single_loop_spans.get_mut(&loop_index).unwrap().clear()
    }
}

#[derive(Debug, Clone)]
/// A LoopSpan is a special snapshot of memory that represents the set of cells
/// which could ever affect the future execution of a given loop at some point
/// in time. See LOOP_SPAN.md for more information.
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
        // consider everything to the left of the touched region to be included.
        // Otherwise include everything to the right of the touched region. If the displacement is
        // zero, then don't include anything extra and just return the touched region as is.
        match self.displacement().cmp(&0) {
            std::cmp::Ordering::Less => &self.memory_at_loop_start[0..=max_index],
            std::cmp::Ordering::Greater => &self.memory_at_loop_start[min_index..],
            std::cmp::Ordering::Equal => &self.memory_at_loop_start[min_index..=max_index],
        }
    }

    fn displacement(&self) -> isize {
        self.current_memory_pointer as isize - self.starting_memory_pointer as isize
    }
}

impl PartialEq for LoopSpan {
    fn eq(&self, other: &Self) -> bool {
        let displacements_match = self.displacement() == other.displacement();
        let masks_match = self.memory_mask() == other.memory_mask();

        displacements_match && masks_match
    }
}

impl Eq for LoopSpan {}

impl Display for LoopSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "start: {} curr: {} min: {} max: {} (disp: {})",
            self.starting_memory_pointer,
            self.current_memory_pointer,
            self.min_index,
            self.max_index,
            self.displacement()
        )?;

        for i in 0..=self.max_index {
            let bg = if i == self.starting_memory_pointer {
                AnsiColors::BrightGreen
            } else if self.min_index <= i && i <= self.max_index {
                AnsiColors::BrightCyan
            } else {
                AnsiColors::Default
            };
            write!(f, "{} ", to_hex(self.memory_at_loop_start[i]).on_color(bg))?;
        }
        writeln!(f)?;

        for i in 0..=self.max_index {
            let text = if i == self.current_memory_pointer {
                "^^"
            } else if i == self.min_index {
                "|-"
            } else if i == self.max_index {
                "-|"
            } else if self.min_index < i && i < self.max_index {
                "--"
            } else {
                "  "
            };

            write!(f, "{} ", text)?;
        }
        writeln!(f)?;

        writeln!(f, "{}", array_to_string(self.memory_mask()))?;
        Ok(())
    }
}

// Transform the u8 to a hexidecimal encoded string
fn to_hex(x: u8) -> String {
    format!("{:0>2X}", x)
}

// Transform the array of u8s to a string of hexidecimal encoded values, seperated by spaces
fn array_to_string(array: &[u8]) -> String {
    array
        .iter()
        .map(|x| to_hex(*x))
        .intersperse(" ".to_string())
        .collect()
}

// Return a string with a specific position highlighted by ^^
fn highlight(index: usize) -> String {
    (0..=index)
        .map(|i| if index == i { "^^" } else { "  " })
        .intersperse(" ")
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Details the current status of execution in an ExecutionContext.
pub enum ExecutionState {
    /// The program has not halted yet, but no infinite loop has been detected
    Running,
    /// The program has halted.
    Halted,
    /// The program has not halted and an infinite loop was detected, indicating
    /// that the program will never halt.
    InfiniteLoop(LoopReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Details how the ExecutionContext detected that a given program will never
/// halt.
pub enum LoopReason {
    /// A LoopIfNonZero instruction was executed, so the program cannot halt.
    LoopIfNonzero,
    /// A loop span cycle was detected between the following LoopSpans.
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
/// A compiled program which can be executed in an ExecutionContext.
pub struct Program {
    pub original_instrs: Vec<Instr>,
    extended_instrs: Vec<ExtendedInstr>,
    // A dictionary mapping start and end loop instructions to each other. The
    // key-value pairs represent the index into extended_instrs for the
    // corresponding start and end loops.
    loop_dict: HashMap<usize, usize>,
}

impl Program {
    /// Create a Program from a list of instructions. If there are mismatched
    /// braces, a CompileError is returned.
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
        write!(f, "{}", Instr::to_string(&self.original_instrs),)
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

/// An extended set of Brainfuck instructions. This is intended to simplify
/// certain common Brainfuck constucts into a single conceptual instruction.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ExtendedInstr {
    /// A base instruction that has not been transformed.
    BaseInstr(Instr),
    /// An instruction which, when executed, causes an infinite loop if the
    /// current memory cell is nonzero, and otherwise is a NOP. This instruction
    /// of length 2, and represents "[]" in base Brainfuck.
    LoopIfNonzero,
}

impl ExtendedInstr {
    /// Transform a list of base Brainfuck instructions into a list of extended
    /// Brainfuck instructions. The following constructs are transformed:
    /// [] -> LoopIfNonzero
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
        match self {
            ExtendedInstr::BaseInstr(base_instr) => write!(f, "{}", base_instr),
            ExtendedInstr::LoopIfNonzero => write!(f, "L"),
        }
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

/// The set of Brainfuck instructions. These are all of the possible
/// instructions in a Brainfuck program, before any optimizations are applied.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Instr {
    Plus,
    Minus,
    Left,
    Right,
    StartLoop,
    EndLoop,
}

impl Instr {
    // Transform a list of instructions into a human readable String.
    pub fn to_string(program: &[Instr]) -> String {
        program.iter().map(|instr| instr.to_string()).collect()
    }
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

/// A compile error specifiying why the given Brainfuck program could not be
/// compiled.
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
