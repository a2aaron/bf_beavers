use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt::Display;

const MEMORY_BEHAVIOR: MemoryBehavior = MemoryBehavior::InfiniteRightwards;
const INITAL_MEMORY: usize = 1;
const EXTEND_MEMORY_AMOUNT: usize = 1;

#[derive(Debug)]
pub struct ExecutionContext {
    memory_behavior: MemoryBehavior,
    memory: Vec<u8>,
    memory_pointer: usize,
    program: Program,
    program_pointer: usize,
}

impl ExecutionContext {
    pub fn new(program: &Program) -> ExecutionContext {
        ExecutionContext {
            memory_behavior: MEMORY_BEHAVIOR,
            memory: vec![0; INITAL_MEMORY],
            memory_pointer: 0,
            program_pointer: 0,
            program: program.clone(),
        }
    }

    pub fn step(&mut self) -> (usize, ExecutionState) {
        use self::MemoryBehavior::*;
        use Instr::*;

        let instruction = self.program.get(self.program_pointer);

        match instruction {
            None => (0, ExecutionState::Halted),
            Some(instruction) => match instruction {
                ExtendedInstr::BaseInstr(instruction) => {
                    match instruction {
                        Plus => {
                            self.memory[self.memory_pointer] =
                                self.memory[self.memory_pointer].wrapping_add(1)
                        }
                        Minus => {
                            self.memory[self.memory_pointer] =
                                self.memory[self.memory_pointer].wrapping_sub(1)
                        }
                        Left => match self.memory_behavior {
                            Wrapping(modulo) => {
                                self.memory_pointer = wrapping_add(self.memory_pointer, -1, modulo)
                            }
                            InfiniteRightwards => {
                                self.memory_pointer = self.memory_pointer.saturating_sub(1)
                            }
                        },
                        Right => match self.memory_behavior {
                            Wrapping(modulo) => {
                                self.memory_pointer = wrapping_add(self.memory_pointer, 1, modulo)
                            }
                            InfiniteRightwards => {
                                self.memory_pointer += 1;
                                if self.memory_pointer >= self.memory.len() {
                                    self.memory.extend([0; EXTEND_MEMORY_AMOUNT].iter());
                                }
                            }
                        },
                        StartLoop => {
                            if self.memory[self.memory_pointer] == 0 {
                                self.program_pointer = self
                                    .program
                                    .matching_loop(self.program_pointer)
                                    .expect("missing StartLoop dict entry!")
                                    + 1;
                            }
                        }
                        EndLoop => {
                            if self.memory[self.memory_pointer] != 0 {
                                self.program_pointer = self
                                    .program
                                    .matching_loop(self.program_pointer)
                                    .expect("missing EndLoop dict entry!");
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
                        (2, ExecutionState::InfiniteLoop)
                    }
                }
            },
        }
    }

    pub fn print_state(&self) {
        let this_instr = if let Some(instr) = self.program.get(self.program_pointer) {
            instr.to_string()
        } else {
            "HALTED".to_string()
        };

        let memory: String = self
            .memory
            .iter()
            .map(|x| format!("{:0>2X}", x))
            .intersperse(" ".to_string())
            .collect();
        println!("[{}] (this_instr = {})", memory, this_instr,);

        let memory_pointer: String = self
            .memory
            .iter()
            .enumerate()
            .map(|(index, _)| {
                if index == self.memory_pointer {
                    "^^"
                } else {
                    "  "
                }
            })
            .intersperse(" ")
            .collect();
        println!(" {} ", memory_pointer);
    }
}

fn wrapping_add(a: usize, b: isize, modulo: usize) -> usize {
    let x = a as isize + b;
    if x < 0 {
        (x + modulo as isize) as usize % modulo
    } else {
        x as usize % modulo
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MemoryBehavior {
    Wrapping(usize),
    InfiniteRightwards,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ExecutionState {
    Running,
    Halted,
    InfiniteLoop,
}

#[derive(Debug, Clone)]
pub struct Program {
    original_instrs: Vec<Instr>,
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
        write!(f, "{}", to_string(&self.original_instrs))
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

#[derive(Debug, Copy, Clone)]
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
        match self {
            ExtendedInstr::BaseInstr(instr) => write!(f, "{}", instr),
            ExtendedInstr::LoopIfNonzero => write!(f, "[]"),
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
