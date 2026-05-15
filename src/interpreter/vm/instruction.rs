use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;

#[derive(ToPrimitive, FromPrimitive)]
pub enum OpCode {
    Return = 0,
    LoadConst,
    Negate,
    Add,
    Subtract,
    Multiply,
    Divide,
}

pub enum Instruction {
    Return,
    Const { offset: u8 },
    Negate,
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl Instruction {
    pub fn from_bytes_iter(iter: &mut impl Iterator<Item = u8>) -> Option<(Self, usize)> {
        let mut offset = 1;
        let instruction = match OpCode::from_u8(iter.next()?)? {
            OpCode::Return => Instruction::Return,
            OpCode::LoadConst => {
                offset += 1;
                let const_offset = iter.next()?;
                Instruction::Const { offset: const_offset }
            },
            OpCode::Negate => Instruction::Negate,
            OpCode::Add => Instruction::Add,
            OpCode::Subtract => Instruction::Subtract,
            OpCode::Multiply => Instruction::Multiply,
            OpCode::Divide => Instruction::Divide,
        };
        Some((instruction, offset))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Instruction::Return => vec![OpCode::Return as u8],
            Instruction::Const { offset } => vec![OpCode::LoadConst as u8, *offset],
            Instruction::Negate => vec![OpCode::Negate as u8],
            Instruction::Add => vec![OpCode::Add as u8],
            Instruction::Subtract => vec![OpCode::Subtract as u8],
            Instruction::Multiply => vec![OpCode::Multiply as u8],
            Instruction::Divide => vec![OpCode::Divide as u8],
        }
    }
}
