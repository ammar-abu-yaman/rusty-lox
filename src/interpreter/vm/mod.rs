use std::io::Write;

mod compiler;
pub mod instruction;
pub mod log;
pub mod mem;
pub mod object;
pub mod result;
pub mod value;

use arrayvec::ArrayVec;
use instruction::Instruction;
use mem::MemoryManager;
use result::{InterpreterError, InterpreterResult};
use value::Value;

use crate::scanner::Scanner;

pub(self) const STACK_SIZE: usize = 256;

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
        let mut logger = log::Logger::new(&mut self.writer);
        let mut compiler = compiler::ByteCodeCompiler::new(&scanner, &mut logger, &mut chunk, &mut self.mem);

        compiler.compile();
        compiler.write_end();
        if compiler.has_error {
            return InterpreterResult::Err(InterpreterError::Compile);
        }

        std::mem::drop(compiler);
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
                self.disassemble(&ctx.chunk, &instruction, ctx.ip, offset);
            }
            match instruction {
                Instruction::Return => return Ok(()),
                Instruction::Const { offset } => {
                    let value = ctx.chunk.constants[offset as usize].clone();
                    ctx.stack.push(value);
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
                        return Err(InterpreterError::Runtime);
                    }
                },
                Instruction::SetGlobal { index } => {
                    let name = ctx.chunk.constants[index as usize].as_object_ptr();
                    if !self.mem.set_global(name, ctx.peek_stack(0).cloned().unwrap()) {
                        let name_str = unsafe { name.as_ref_unchecked().str() };
                        self.runtime_err(&mut ctx, &format!("Undefined global: {name_str}"));
                        return Err(InterpreterError::Runtime);
                    }
                },
                Instruction::GetLocal { index } => {
                    let value = ctx.stack[index as usize].clone();
                    ctx.stack.push(value);
                },
                Instruction::SetLocal { index } => {
                    let value = ctx.stack.last().cloned().unwrap();
                    ctx.stack[index as usize] = value;
                },
                Instruction::JumpIfFalse { offset: jump_offset } => {
                    if ctx.peek_stack(0).map_or(true, |v| v.is_falsy()) {
                        ctx.ip += jump_offset as usize;
                    }
                },
                Instruction::Jump { offset: jump_offset } => {
                    ctx.ip += jump_offset as usize;
                },
                Instruction::Loop { offset: jump_offset } => {
                    ctx.ip -= jump_offset as usize;
                },
            }

            ctx.ip += offset;
        }
        Ok(())
    }

    #[inline(always)]
    fn binary_math_op(&mut self, ctx: &mut RunContext, op: fn(f64, f64) -> f64) -> InterpreterResult<()> {
        if !matches!(ctx.peek_stack(0), Some(Value::Number(..))) || !matches!(ctx.peek_stack(1), Some(Value::Number(..))) {
            self.runtime_err(ctx, "Operand must be a number.");
            return Err(InterpreterError::Runtime);
        }

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

    fn disassemble(&mut self, chunk: &Chunk, instruction: &Instruction, offset: usize, consumed: usize) {
        write!(self.writer, "{:04} ", offset);

        // Line column logic
        if offset > 0 && chunk.lines[offset] == chunk.lines[offset - 1] {
            write!(self.writer, "   | ");
        } else {
            write!(self.writer, "{:4} ", chunk.lines[offset]);
        }

        match instruction {
            Instruction::Return => {
                writeln!(self.writer, "OP_RETURN");
            },
            Instruction::Const { offset: const_offset } => {
                let value = &chunk.constants[*const_offset as usize];
                writeln!(self.writer, "{:<16} {:4} '{}'", "OP_CONSTANT", const_offset, value);
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
            Instruction::DefineGlobal { index } => {
                let value = &chunk.constants[*index as usize];
                writeln!(self.writer, "{:<16} {:4} '{}'", "OP_DEFINE_GLOBAL", index, value);
            },
            Instruction::GetGlobal { index } => {
                let value = &chunk.constants[*index as usize];
                writeln!(self.writer, "{:<16} {:4} '{}'", "OP_GET_GLOBAL", index, value);
            },
            Instruction::SetGlobal { index } => {
                let value = &chunk.constants[*index as usize];
                writeln!(self.writer, "{:<16} {:4} '{}'", "OP_SET_GLOBAL", index, value);
            },
            Instruction::GetLocal { index } => {
                writeln!(self.writer, "{:<16} {:4}", "OP_GET_LOCAL", index);
            },
            Instruction::SetLocal { index } => {
                writeln!(self.writer, "{:<16} {:4}", "OP_SET_LOCAL", index);
            },
            Instruction::JumpIfFalse { offset: jump_offset } => {
                let target = offset + consumed + *jump_offset as usize;
                writeln!(self.writer, "{:<16} {:4} -> {:04}", "OP_JUMP_IF_FALSE", jump_offset, target);
            },
            Instruction::Jump { offset: jump_offset } => {
                let target = offset + consumed + *jump_offset as usize;
                writeln!(self.writer, "{:<16} {:4} -> {:04}", "OP_JUMP", jump_offset, target);
            },
            Instruction::Loop { offset: jump_offset } => {
                let target = offset + consumed + *jump_offset as usize;
                writeln!(self.writer, "{:<16} {:4} -> {:04}", "OP_LOOP", jump_offset, target);
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
    use instruction::OpCode;

    fn run_chunk(chunk: Chunk) -> (InterpreterResult<()>, String) {
        let mut buffer = Vec::new();
        let result;
        {
            let mut vm = VirtualMachine::new(false, &mut buffer);
            result = vm.run(chunk);
        }
        (result, String::from_utf8_lossy(&buffer).to_string())
    }

    fn run_chunk_with_mem(chunk: Chunk, mem: MemoryManager) -> (InterpreterResult<()>, String, MemoryManager) {
        let mut buffer = Vec::new();
        let result;
        let returned_mem;
        {
            let mut vm = VirtualMachine {
                debug: false,
                writer: &mut buffer,
                mem,
            };
            result = vm.run(chunk);
            returned_mem = vm.mem; // Extract the memory manager to inspect globals/heap after run
        }
        (result, String::from_utf8_lossy(&buffer).to_string(), returned_mem)
    }

    #[test]
    fn test_arithmetic() {
        let mut chunk = Chunk::new();
        chunk.constants.push(Value::Number(1.2));
        chunk.constants.push(Value::Number(3.4));

        chunk.code.push(OpCode::LoadConst as u8);
        chunk.code.push(0);
        chunk.code.push(OpCode::LoadConst as u8);
        chunk.code.push(1);
        chunk.code.push(OpCode::Add as u8);
        chunk.code.push(OpCode::Print as u8);
        chunk.code.push(OpCode::Return as u8);
        chunk.lines = vec![1; chunk.code.len()];

        let (result, output) = run_chunk(chunk);
        assert!(result.is_ok());
        assert_eq!(output.trim(), "4.6");
    }

    #[test]
    fn test_negate() {
        let mut chunk = Chunk::new();
        chunk.constants.push(Value::Number(10.0));

        chunk.code.push(OpCode::LoadConst as u8);
        chunk.code.push(0);
        chunk.code.push(OpCode::Negate as u8);
        chunk.code.push(OpCode::Print as u8);
        chunk.code.push(OpCode::Return as u8);
        chunk.lines = vec![1; chunk.code.len()];

        let (result, output) = run_chunk(chunk);
        assert!(result.is_ok());
        assert_eq!(output.trim(), "-10");
    }

    #[test]
    fn test_literals() {
        let mut chunk = Chunk::new();
        chunk.code.push(OpCode::LoadTrue as u8);
        chunk.code.push(OpCode::Print as u8);
        chunk.code.push(OpCode::LoadFalse as u8);
        chunk.code.push(OpCode::Print as u8);
        chunk.code.push(OpCode::LoadNil as u8);
        chunk.code.push(OpCode::Print as u8);
        chunk.code.push(OpCode::Return as u8);
        chunk.lines = vec![1; chunk.code.len()];

        let (result, output) = run_chunk(chunk);
        assert!(result.is_ok());
        assert_eq!(output.trim(), "true\nfalse\nnil");
    }

    #[test]
    fn test_comparison() {
        let mut chunk = Chunk::new();
        chunk.constants.push(Value::Number(5.0));
        chunk.constants.push(Value::Number(10.0));

        // 5 < 10
        chunk.code.push(OpCode::LoadConst as u8);
        chunk.code.push(0);
        chunk.code.push(OpCode::LoadConst as u8);
        chunk.code.push(1);
        chunk.code.push(OpCode::Less as u8);
        chunk.code.push(OpCode::Print as u8);

        // 5 > 10
        chunk.code.push(OpCode::LoadConst as u8);
        chunk.code.push(0);
        chunk.code.push(OpCode::LoadConst as u8);
        chunk.code.push(1);
        chunk.code.push(OpCode::Greater as u8);
        chunk.code.push(OpCode::Print as u8);

        chunk.code.push(OpCode::Return as u8);
        chunk.lines = vec![1; chunk.code.len()];

        let (result, output) = run_chunk(chunk);
        assert!(result.is_ok());
        assert_eq!(output.trim(), "true\nfalse");
    }

    #[test]
    fn test_global_variables() {
        let mut mm = MemoryManager::new();
        let name_ptr = mm.intern_string("a");

        let mut chunk = Chunk::new();
        chunk.constants.push(Value::Object(name_ptr));
        chunk.constants.push(Value::Number(100.0));
        chunk.constants.push(Value::Number(200.0));

        // var a = 100;
        chunk.code.push(OpCode::LoadConst as u8);
        chunk.code.push(1);
        chunk.code.push(OpCode::DefineGlobal as u8);
        chunk.code.push(0);

        // a = 200;
        chunk.code.push(OpCode::LoadConst as u8);
        chunk.code.push(2);
        chunk.code.push(OpCode::SetGlobal as u8);
        chunk.code.push(0);
        chunk.code.push(OpCode::Pop as u8); // pop assignment value

        // print a;
        chunk.code.push(OpCode::GetGlobal as u8);
        chunk.code.push(0);
        chunk.code.push(OpCode::Print as u8);

        chunk.code.push(OpCode::Return as u8);
        chunk.lines = vec![1; chunk.code.len()];

        let (result, output, _) = run_chunk_with_mem(chunk, mm);
        assert!(result.is_ok());
        assert_eq!(output.trim(), "200");
    }

    #[test]
    fn test_local_variables() {
        let mut chunk = Chunk::new();
        chunk.constants.push(Value::Number(10.0));
        chunk.constants.push(Value::Number(20.0));

        // push 10 (local 0)
        chunk.code.push(OpCode::LoadConst as u8);
        chunk.code.push(0);
        // push 20 (local 1)
        chunk.code.push(OpCode::LoadConst as u8);
        chunk.code.push(1);

        // set local 0 = 30 (manually push 30 first)
        chunk.constants.push(Value::Number(30.0));
        chunk.code.push(OpCode::LoadConst as u8);
        chunk.code.push(2);
        chunk.code.push(OpCode::SetLocal as u8);
        chunk.code.push(0);
        chunk.code.push(OpCode::Pop as u8);

        // print local 0
        chunk.code.push(OpCode::GetLocal as u8);
        chunk.code.push(0);
        chunk.code.push(OpCode::Print as u8);

        // print local 1
        chunk.code.push(OpCode::GetLocal as u8);
        chunk.code.push(1);
        chunk.code.push(OpCode::Print as u8);

        chunk.code.push(OpCode::Return as u8);
        chunk.lines = vec![1; chunk.code.len()];

        let (result, output) = run_chunk(chunk);
        assert!(result.is_ok());
        assert_eq!(output.trim(), "30\n20");
    }

    #[test]
    fn test_blocks_and_scopes() {
        let mut chunk = Chunk::new();
        chunk.constants.push(Value::Number(1.0));
        chunk.constants.push(Value::Number(2.0));

        // Outer scope: var a = 1
        chunk.code.push(OpCode::LoadConst as u8);
        chunk.code.push(0);

        // Inner block
        // var b = 2
        chunk.code.push(OpCode::LoadConst as u8);
        chunk.code.push(1);
        // print b
        chunk.code.push(OpCode::GetLocal as u8);
        chunk.code.push(1);
        chunk.code.push(OpCode::Print as u8);
        // end block: pop b
        chunk.code.push(OpCode::Pop as u8);

        // back in outer scope
        // print a
        chunk.code.push(OpCode::GetLocal as u8);
        chunk.code.push(0);
        chunk.code.push(OpCode::Print as u8);

        chunk.code.push(OpCode::Return as u8);
        chunk.lines = vec![1; chunk.code.len()];

        let (result, output) = run_chunk(chunk);
        assert!(result.is_ok());
        assert_eq!(output.trim(), "2\n1");
    }
}
