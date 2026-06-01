use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;

#[derive(Debug, ToPrimitive, FromPrimitive)]
pub enum OpCode {
    Return = 0,
    LoadConst,
    LoadTrue,
    LoadFalse,
    LoadNil,
    Negate,
    Add,
    Subtract,
    Multiply,
    Divide,
    Not,
    Equal,
    Greater,
    Less,
    Print,
    Pop,
    DefineGlobal,
    GetGlobal,
    SetGlobal,
}

impl From<OpCode> for u8 {
    fn from(op: OpCode) -> Self {
        op as u8
    }
}

#[derive(Debug, PartialEq)]
pub enum Instruction {
    Return,
    Const { offset: u8 },
    Negate,
    Not,
    Add,
    Subtract,
    Multiply,
    Divide,
    LoadTrue,
    LoadFalse,
    LoadNil,
    Equal,
    Greater,
    Less,
    Print,
    Pop,
    DefineGlobal { index: u8 },
    GetGlobal { index: u8 },
    SetGlobal { index: u8 },
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
            OpCode::LoadTrue => Instruction::LoadTrue,
            OpCode::LoadFalse => Instruction::LoadFalse,
            OpCode::LoadNil => Instruction::LoadNil,
            OpCode::Not => Instruction::Not,
            OpCode::Equal => Instruction::Equal,
            OpCode::Greater => Instruction::Greater,
            OpCode::Less => Instruction::Less,
            OpCode::Print => Instruction::Print,
            OpCode::Pop => Instruction::Pop,
            OpCode::DefineGlobal => {
                let index = iter.next()?;
                offset += 1;
                Instruction::DefineGlobal { index }
            },
            OpCode::GetGlobal => {
                let index = iter.next()?;
                offset += 1;
                Instruction::GetGlobal { index }
            },
            OpCode::SetGlobal => {
                let index = iter.next()?;
                offset += 1;
                Instruction::SetGlobal { index }
            },
        };
        Some((instruction, offset))
    }
}
