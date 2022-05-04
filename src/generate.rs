use std::{iter::successors, ops::Range};

use crate::bf::{Instr, Program};

pub fn brute_force_chain(lengths: Range<usize>) -> impl Iterator<Item = Program> {
    lengths.into_iter().flat_map(brute_force_iterator)
}

pub fn brute_force_iterator(length: usize) -> impl Iterator<Item = Program> {
    lexiographic_order(length).filter_map(|instrs| Program::new(&instrs).ok())
}

pub fn lexiographic_order(length: usize) -> impl Iterator<Item = Vec<Instr>> {
    fn next(instr: &Instr) -> (bool, Instr) {
        match instr {
            Instr::Plus => (false, Instr::Minus),
            Instr::Minus => (false, Instr::Left),
            Instr::Left => (false, Instr::Right),
            Instr::Right => (false, Instr::StartLoop),
            Instr::StartLoop => (false, Instr::EndLoop),
            Instr::EndLoop => (true, Instr::Plus),
        }
    }

    fn next_program(program: &[Instr]) -> Option<Vec<Instr>> {
        let mut next_program = program.to_vec();
        let mut wrap_count = 0;
        for instr in next_program.iter_mut().rev() {
            let (did_wrap, next_instr) = next(instr);
            *instr = next_instr;
            if !did_wrap {
                break;
            } else {
                wrap_count += 1;
            }
        }
        if wrap_count == program.len() {
            None
        } else {
            Some(next_program)
        }
    }

    let starting_program = if length == 0 {
        None
    } else {
        Some(vec![Instr::Plus; length])
    };
    successors(starting_program, |this_program| next_program(this_program))
}

// enum Node {
//     // A "leaf node", representing one of either +, -, <, or >
//     Leaf(bf::Instr),
//     // A node representing a loop. Represents [*], where * is some BF subprogram
//     Loop(Vec<Node>),
// }

// struct BFTree {
//     root: Vec<Node>,
// }

// impl From<bf::Program> for BFTree {
//     fn from(program: bf::Program) -> Self {
//         let mut root_nodes = vec![];
//         for ()

//         BFTree {
//             root: root_nodes
//         }
//     }
// }
