use std::io::Write;

pub mod instruction;
pub mod result;
pub mod value;

use arrayvec::ArrayVec;
pub use instruction::Instruction;
use result::InterpreterResult;
pub use value::Value;

const STACK_SIZE: usize = 256;

pub struct VirtualMachine<W: Write> {
    debug: bool,
    writer: W,
}

impl<W: Write> VirtualMachine<W> {
    pub fn new(debug: bool, writer: W) -> Self {
        Self { debug, writer }
    }

    pub fn interpret(&mut self, chunk: Chunk) -> InterpreterResult<()> {
        let ctx = RunContext {
            chunk,
            stack: ArrayVec::new(),
            ip: 0,
        };

        self.run(ctx)
    }

    fn run(&mut self, mut ctx: RunContext) -> InterpreterResult<()> {
        let mut iter = ctx.chunk.code.iter().copied();
        while let Some((instruction, offset)) = Instruction::from_bytes_iter(&mut iter) {
            if self.debug {
                writeln!(self.writer, "stack: {:?}", ctx.stack);
                self.disassemble(&instruction, None, ctx.ip);
            }
            ctx.ip += offset;
            match instruction {
                Instruction::Return => return Ok(()),
                Instruction::Const { offset } => {
                    let value = ctx.chunk.constants[offset as usize].clone();
                    ctx.stack.push(value);
                    return Ok(());
                },
                Instruction::Negate => {
                    let value = ctx.stack.pop().unwrap().as_number().expect("expected number");
                    let result = Value::Number(-value);
                    ctx.stack.push(result);
                },
                Instruction::Add => self.binary_math_op(&mut ctx.stack, |a, b| a + b)?,
                Instruction::Subtract => self.binary_math_op(&mut ctx.stack, |a, b| a - b)?,
                Instruction::Multiply => self.binary_math_op(&mut ctx.stack, |a, b| a * b)?,
                Instruction::Divide => self.binary_math_op(&mut ctx.stack, |a, b| a / b)?,
            }
        }
        Ok(())
    }

    #[inline(always)]
    fn binary_math_op(&mut self, stack: &mut ArrayVec<Value, STACK_SIZE>, op: fn(f64, f64) -> f64) -> InterpreterResult<()> {
        let value2 = stack.pop().unwrap().as_number().expect("expected number");
        let value1 = stack.pop().unwrap().as_number().expect("expected number");
        let result = Value::Number(op(value1, value2));
        stack.push(result);
        Ok(())
    }

    fn disassemble(&mut self, instruction: &Instruction, value: Option<&Value>, offset: usize) {
        print!("{:04} ", offset);
        match instruction {
            Instruction::Return => {
                writeln!(self.writer, "OP_RETURN");
            },
            Instruction::Const { offset: const_offset } => {
                write!(self.writer, "{:<16} {:4}", "OP_CONSTANT", const_offset);
                if let Some(v) = value {
                    write!(self.writer, " '{v}'");
                }
                writeln!(self.writer);
            },
            Instruction::Negate => {
                writeln!(self.writer, "OP_NEGATE");
            },
            Instruction::Add => {
                writeln!(self.writer, "OP_ADD");
            },
            Instruction::Subtract => {
                writeln!(self.writer, "OP_SUBTRACT");
            },
            Instruction::Multiply => {
                writeln!(self.writer, "OP_MULTIPLY");
            },
            Instruction::Divide => {
                writeln!(self.writer, "OP_DIVIDE");
            },
        }
    }
}

struct RunContext {
    chunk: Chunk,
    stack: ArrayVec<Value, STACK_SIZE>,
    ip: usize,
}

pub struct Chunk {
    pub code: Vec<u8>,
    pub lines: Vec<u32>,
    pub constants: Vec<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
}
