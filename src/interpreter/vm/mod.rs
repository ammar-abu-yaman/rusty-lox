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
use object::{native_clock, NativeFunction};
use result::{InterpreterError, InterpreterResult};
use value::Value;

use crate::{
    interpreter::vm::object::{Object, ObjectData},
    scanner::Scanner,
};

pub(self) const MAX_FRAMES: usize = 64;
pub(self) const STACK_SIZE: usize = (u8::MAX as usize + 1);
pub(self) const MAX_STACK: usize = MAX_FRAMES * STACK_SIZE;

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
        let mut logger = log::Logger::new(&mut self.writer);
        let mut compiler = compiler::ByteCodeCompiler::new(&scanner, &mut logger, &mut self.mem);

        compiler.compile();
        compiler.write_end();
        if compiler.has_error {
            return InterpreterResult::Err(InterpreterError::Compile);
        }

        let function = compiler.end_compilation();
        std::mem::drop(compiler);
        self.run(function)?;
        Ok(())
    }

    pub fn run(&mut self, function: *mut Object) -> InterpreterResult<()> {
        let mut ctx = RunContext::new();
        self.define_native(&mut ctx, "clock", native_clock);

        ctx.frames.push(CallFrame {
            function,
            ip: 0,
            slots_offest: ctx.stack.len(),
        });
        ctx.stack.push(Value::Object(function));

        loop {
            let instruction_result = {
                let ip = ctx.frame().ip;
                Instruction::from_bytes_iter(&mut ctx.frame_mut().chunk_mut().code.iter().skip(ip).copied())
            };

            if instruction_result.is_none() {
                break;
            }
            let (instruction, offset) = instruction_result.unwrap();
            if self.debug {
                self.disassemble(&ctx.frame().chunk(), &instruction, ctx.frame().ip, offset);
            }

            ctx.frame_mut().ip += offset;
            match instruction {
                Instruction::Const { offset } => {
                    let value = ctx.frame().chunk().constants[offset as usize].clone();
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
                    let name = ctx.frame().chunk().constants[index as usize].as_object_ptr();
                    let value = ctx.stack.pop().unwrap();
                    self.mem.define_global(name, value);
                },
                Instruction::GetGlobal { index } => {
                    let name = ctx.frame().chunk().constants[index as usize].as_object_ptr();
                    if let Some(value) = self.mem.get_global(name) {
                        ctx.stack.push(value);
                    } else {
                        let name_str = unsafe { name.as_ref_unchecked().as_str() };
                        self.runtime_err(&mut ctx, &format!("Undefined global: {name_str}"));
                        return Err(InterpreterError::Runtime);
                    }
                },
                Instruction::SetGlobal { index } => {
                    let name = ctx.frame().chunk().constants[index as usize].as_object_ptr();
                    if !self.mem.set_global(name, ctx.peek_stack(0).cloned().unwrap()) {
                        let name_str = unsafe { name.as_ref_unchecked().as_str() };
                        self.runtime_err(&mut ctx, &format!("Undefined global: {name_str}"));
                        return Err(InterpreterError::Runtime);
                    }
                },
                Instruction::GetLocal { index } => {
                    let slots_offset = ctx.frame().slots_offest;
                    let value = ctx.stack[slots_offset + index as usize].clone();
                    ctx.stack.push(value);
                },
                Instruction::SetLocal { index } => {
                    let slots_offset = ctx.frame().slots_offest;
                    let value = ctx.stack.last().cloned().unwrap();
                    ctx.stack[slots_offset + index as usize] = value;
                },
                Instruction::JumpIfFalse { offset: jump_offset } => {
                    if ctx.peek_stack(0).map_or(true, |v| v.is_falsy()) {
                        ctx.frame_mut().ip += jump_offset as usize;
                    }
                },
                Instruction::Jump { offset: jump_offset } => {
                    ctx.frame_mut().ip += jump_offset as usize;
                },
                Instruction::Loop { offset: jump_offset } => {
                    ctx.frame_mut().ip -= jump_offset as usize;
                },
                Instruction::Call { arg_count } => {
                    if !self.call_value(ctx.peek_stack(arg_count as usize).cloned().unwrap(), &mut ctx, arg_count) {
                        return Err(InterpreterError::Runtime);
                    }
                },
                Instruction::Return => {
                    let result = ctx.stack.pop().unwrap();
                    if ctx.frames.len() == 1 {
                        ctx.stack.pop();
                        return Ok(());
                    }

                    ctx.pop_frame();
                    ctx.stack.push(result);
                },
            }
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
        let str1 = value1.as_object().as_str();
        let str2 = value2.as_object().as_str();
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

    fn call_value(&mut self, value: Value, ctx: &mut RunContext, arg_count: u8) -> bool {
        if let Value::Object(obj) = value {
            let obj = unsafe { &mut *obj };
            match obj {
                Object {
                    data: ObjectData::Function(..),
                    ..
                } => return self.call(obj, ctx, arg_count),
                Object {
                    data: ObjectData::NativeFunction(native_fn),
                    ..
                } => {
                    let result = native_fn(&ctx.stack[&ctx.stack.len() - arg_count as usize..]);
                    ctx.pop_len((arg_count + 1).into());
                    ctx.stack.push(result);
                    return true;
                },
                _ => {},
            }
        }

        self.runtime_err(ctx, "Can only call functions and classes.");
        return false;
    }

    fn call(&mut self, obj: &mut Object, ctx: &mut RunContext, arg_count: u8) -> bool {
        if obj.as_function().arity != arg_count as usize {
            self.runtime_err(ctx, &format!("Expected {} arguments, got {}.", obj.as_function().arity, arg_count));
            return false;
        }
        if ctx.frames.len() == MAX_FRAMES {
            self.runtime_err(ctx, "Stack overflow.");
            return false;
        }

        let frame = CallFrame {
            function: obj,
            ip: 0,
            slots_offest: ctx.stack.len() - arg_count as usize - 1,
        };
        ctx.frames.push(frame);
        return true;
    }

    fn define_native(&mut self, ctx: &mut RunContext, name: &str, native_fn: NativeFunction) {
        ctx.stack.push(Value::Object(self.mem.intern_string(name)));
        ctx.stack.push(Value::Object(self.mem.allocate_native_function(native_fn)));
        self.mem.set_global(ctx.stack[0].as_object_ptr(), ctx.stack[1]);
        ctx.stack.pop();
        ctx.stack.pop();
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
                let target = offset + consumed - *jump_offset as usize;
                writeln!(self.writer, "{:<16} {:4} -> {:04}", "OP_LOOP", jump_offset, target);
            },
            Instruction::Call { arg_count } => {
                writeln!(self.writer, "{:<16} {:4}", "OP_CALL", arg_count);
            },
        }
    }
}

impl<T: Write> VirtualMachine<T> {
    fn runtime_err(&mut self, ctx: &mut RunContext, message: &str) {
        writeln!(self.writer, "{message}");
        for frame in ctx.frames.iter().rev() {
            let line = frame.chunk().lines[frame.ip];
            let name = unsafe { frame.function().as_function().name.as_ref().map_or("script", |n| n.as_str()) };
            writeln!(self.writer, "[line {line}] in {name}");
        }

        unsafe { ctx.stack.set_len(0) };
    }
}

struct RunContext {
    stack: ArrayVec<Value, MAX_STACK>,
    frames: ArrayVec<CallFrame, MAX_FRAMES>,
}

impl RunContext {
    pub fn new() -> Self {
        Self {
            stack: ArrayVec::new(),
            frames: ArrayVec::new(),
        }
    }

    #[inline(always)]
    pub fn peek_stack(&self, index: usize) -> Option<&Value> {
        self.stack.iter().rev().nth(index)
    }

    #[inline(always)]
    fn frame(&self) -> &CallFrame {
        &self.frames.last().unwrap()
    }

    #[inline(always)]
    fn frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }

    #[inline(always)]
    fn pop_frame(&mut self) -> CallFrame {
        let frame = self.frames.pop().unwrap();
        unsafe {
            self.stack.set_len(frame.slots_offest);
        }

        return frame;
    }

    fn pop_len(&mut self, count: usize) {
        unsafe {
            self.stack.set_len(self.stack.len() - count);
        }
    }
}

#[derive(Debug, Clone, Default)]
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

struct CallFrame {
    function: *mut Object,
    ip: usize,
    slots_offest: usize,
}

impl CallFrame {
    #[inline(always)]
    pub fn chunk(&self) -> &Chunk {
        match &self.function().data {
            ObjectData::Function(f) => &f.chunk,
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    pub fn chunk_mut(&mut self) -> &mut Chunk {
        match &mut self.function_mut().data {
            ObjectData::Function(f) => &mut f.chunk,
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    pub fn function(&self) -> &Object {
        unsafe { self.function.as_ref_unchecked() }
    }

    #[inline(always)]
    pub fn function_mut(&mut self) -> &mut Object {
        unsafe { self.function.as_mut_unchecked() }
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
            let mut mem = MemoryManager::new();
            let name = mem.intern_string("");
            let function = mem.allocate_function(name, 0);
            unsafe {
                if let ObjectData::Function(ref mut f) = (*function).data {
                    f.chunk = chunk;
                }
            }
            let mut vm = VirtualMachine {
                debug: false,
                writer: &mut buffer,
                mem,
            };
            result = vm.run(function);
        }
        (result, String::from_utf8_lossy(&buffer).to_string())
    }

    fn run_chunk_with_mem(chunk: Chunk, mut mem: MemoryManager) -> (InterpreterResult<()>, String, MemoryManager) {
        let mut buffer = Vec::new();
        let result;
        let returned_mem;
        {
            let name = mem.intern_string("");
            let function = mem.allocate_function(name, 0);
            unsafe {
                if let ObjectData::Function(ref mut f) = (*function).data {
                    f.chunk = chunk;
                }
            }
            let mut vm = VirtualMachine {
                debug: false,
                writer: &mut buffer,
                mem,
            };
            result = vm.run(function);
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
