use std::io::Write;

mod compiler;
pub mod instruction;
pub mod log;
pub mod mem;
pub mod object;
pub mod result;
pub mod value;

use arrayvec::ArrayVec;
use instruction::{Instruction, OpCode};
use mem::MemoryManager;
use result::{InterpreterError, InterpreterResult};
use value::Value;

use crate::scanner::Scanner;

const STACK_SIZE: usize = 256;

pub struct VirtualMachine<W: Write> {
    debug: bool,
    writer: W,
    mem: MemoryManager,
}

impl<W: Write> VirtualMachine<W> {
    pub fn new(debug: bool, writer: W) -> Self {
        Self {
            debug,
            writer,
            mem: MemoryManager::new(),
        }
    }

    pub fn interpret(&mut self, source: &str) -> InterpreterResult<()> {
        let scanner = Scanner::new(source);
        let mut chunk = Chunk::new();
        let mut logger = log::Logger;
        let mut compiler = compiler::ByteCodeCompiler::new(&scanner, &mut logger, &mut chunk, &mut self.mem);

        compiler.compile();
        compiler.write_end();
        if compiler.has_error {
            return InterpreterResult::Err(InterpreterError::Compile);
        }

        self.run(chunk)?;
        Ok(())
    }

    pub fn run(&mut self, chunk: Chunk) -> InterpreterResult<()> {
        let mut ctx = RunContext::new(chunk);
        loop {
            let instruction_result = Instruction::from_bytes_iter(&mut ctx.chunk.code.iter().skip(ctx.ip).copied());
            if instruction_result.is_none() {
                break;
            }
            let (instruction, offset) = instruction_result.unwrap();
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
                    if !matches!(ctx.peek_stack(0), Some(Value::Number(..))) {
                        self.runtime_err(&mut ctx, "Operand must be a number.");
                        return Err(InterpreterError::Runtime);
                    }
                    let value = ctx.stack.pop().unwrap().as_number();
                    let result = Value::Number(-value);
                    ctx.stack.push(result);
                },
                Instruction::Add => {
                    if ctx.peek_stack(0).map_or(false, |a| a.is_number()) && ctx.peek_stack(1).map_or(false, |a| a.is_number()) {
                        self.binary_math_op(&mut ctx, |a, b| a + b)?;
                    } else if ctx.peek_stack(0).map_or(false, |a| a.is_string_object()) && ctx.peek_stack(1).map_or(false, |a| a.is_string_object()) {
                        self.concatentate(&mut ctx);
                    } else {
                        self.runtime_err(&mut ctx, "Operands must be numbers or strings.");
                        return Err(InterpreterError::Runtime);
                    }
                },
                Instruction::Subtract => self.binary_math_op(&mut ctx, |a, b| a - b)?,
                Instruction::Multiply => self.binary_math_op(&mut ctx, |a, b| a * b)?,
                Instruction::Divide => self.binary_math_op(&mut ctx, |a, b| a / b)?,
                Instruction::LoadTrue => ctx.stack.push(Value::Bool(true)),
                Instruction::LoadFalse => ctx.stack.push(Value::Bool(false)),
                Instruction::LoadNil => ctx.stack.push(Value::Nil),
                Instruction::Not => {
                    let value = ctx.stack.pop().unwrap();
                    ctx.stack.push(Value::Bool(value.is_falsy()));
                },
                Instruction::Equal => {
                    let b = ctx.stack.pop().unwrap();
                    let a = ctx.stack.pop().unwrap();
                    ctx.stack.push(Value::Bool(a == b));
                    if matches!(a, Value::Object(_)) && matches!(b, Value::Object(_)) {
                        let a = a.as_object();
                        let b = b.as_object();
                        ctx.stack.push(Value::Bool(a == b));
                    } else {
                        ctx.stack.push(Value::Bool(a == b));
                    }
                },
                Instruction::Greater => self.binary_cmp_op(&mut ctx, |a, b| a > b)?,
                Instruction::Less => self.binary_cmp_op(&mut ctx, |a, b| a < b)?,
                Instruction::Print => {
                    let value = ctx.stack.pop().unwrap();
                    writeln!(self.writer, "{}", value);
                },
                Instruction::Pop => {
                    ctx.stack.pop();
                },
                Instruction::DefineGlobal { index } => {
                    let name = ctx.chunk.constants[index as usize].as_object_ptr();
                    let value = ctx.stack.pop().unwrap();
                    self.mem.define_global(name, value);
                },
                Instruction::GetGlobal { index } => {
                    let name = ctx.chunk.constants[index as usize].as_object_ptr();
                    if let Some(value) = self.mem.get_global(name) {
                        ctx.stack.push(value);
                    } else {
                        let name_str = unsafe { name.as_ref_unchecked().str() };
                        self.runtime_err(&mut ctx, &format!("Undefined global: {name_str}"));
                    }
                },
                Instruction::SetGlobal { index } => {
                    let name = ctx.chunk.constants[index as usize].as_object_ptr();
                    if !self.mem.set_global(name, ctx.chunk.constants.pop().unwrap()) {
                        let name_str = unsafe { name.as_ref_unchecked().str() };
                        self.runtime_err(&mut ctx, &format!("Undefined global: {name_str}"));
                    }
                },
            }
        }
        Ok(())
    }

    #[inline(always)]
    fn binary_math_op(&mut self, ctx: &mut RunContext, op: fn(f64, f64) -> f64) -> InterpreterResult<()> {
        let value2 = ctx.stack.pop().unwrap().as_number();
        let value1 = ctx.stack.pop().unwrap().as_number();
        let result = Value::Number(op(value1, value2));
        ctx.stack.push(result);
        Ok(())
    }

    fn concatentate(&mut self, ctx: &mut RunContext) {
        let value2 = ctx.stack.pop().unwrap();
        let value1 = ctx.stack.pop().unwrap();
        let str1 = value1.as_object().str();
        let str2 = value2.as_object().str();
        let result = Value::Object(self.mem.allocate_string(str1.to_owned() + str2));
        ctx.stack.push(result);
    }

    #[inline(always)]
    fn binary_cmp_op(&mut self, ctx: &mut RunContext, op: fn(f64, f64) -> bool) -> InterpreterResult<()> {
        if !matches!(ctx.peek_stack(0), Some(Value::Number(..))) || !matches!(ctx.peek_stack(1), Some(Value::Number(..))) {
            self.runtime_err(ctx, "Operand must be a number.");
            return Err(InterpreterError::Runtime);
        }
        let value2 = ctx.stack.pop().unwrap().as_number();
        let value1 = ctx.stack.pop().unwrap().as_number();
        let result = Value::Bool(op(value1, value2));
        ctx.stack.push(result);
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
            Instruction::LoadTrue => {
                writeln!(self.writer, "OP_TRUE");
            },
            Instruction::LoadFalse => {
                writeln!(self.writer, "OP_FALSE");
            },
            Instruction::LoadNil => {
                writeln!(self.writer, "OP_NIL");
            },
            Instruction::Not => {
                writeln!(self.writer, "OP_NOT");
            },
            Instruction::Equal => {
                writeln!(self.writer, "OP_EQUAL");
            },
            Instruction::Greater => {
                writeln!(self.writer, "OP_GREATER");
            },
            Instruction::Less => {
                writeln!(self.writer, "OP_LESS");
            },
            Instruction::Print => {
                writeln!(self.writer, "OP_PRINT");
            },
            Instruction::Pop => {
                writeln!(self.writer, "OP_POP");
            },
            Instruction::DefineGlobal { .. } => {
                writeln!(self.writer, "OP_DEFINE_GLOBAL");
            },
            Instruction::GetGlobal { .. } => {
                writeln!(self.writer, "OP_GET_GLOBAL");
            },
            Instruction::SetGlobal { .. } => {
                writeln!(self.writer, "OP_SET_GLOBAL");
            },
        }
    }
}

impl<T: Write> VirtualMachine<T> {
    fn runtime_err(&mut self, ctx: &mut RunContext, message: &str) {
        let line = ctx.chunk.lines[ctx.ip];
        writeln!(self.writer, "{message}\n[line {line}] in script");
        unsafe { ctx.stack.set_len(0) };
    }
}

struct RunContext {
    chunk: Chunk,
    stack: ArrayVec<Value, STACK_SIZE>,
    ip: usize,
}

impl RunContext {
    pub fn new(chunk: Chunk) -> Self {
        Self {
            chunk,
            stack: ArrayVec::new(),
            ip: 0,
        }
    }

    pub fn peek_stack(&self, index: usize) -> Option<&Value> {
        self.stack.iter().rev().nth(index)
    }
}

pub struct Chunk {
    pub code: Vec<u8>,
    pub lines: Vec<u32>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            lines: Vec::new(),
            constants: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
