use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::rc::Rc;

#[derive(Debug)]
pub enum CompileError {
    SyntaxError(&'static str),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CompileError::SyntaxError(e) => write!(f, "{}", e),
        }
    }
}

impl Error for CompileError {}

#[derive(Debug)]
pub enum ExecError {
    InvalidInstruction,
    StackEmpty,
    StackFull,
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match *self {
            ExecError::InvalidInstruction => "Invalid instruction",
            ExecError::StackEmpty => "Stack empty",
            ExecError::StackFull => "Stack full",
        };
        write!(f, "{}", msg)
    }
}

impl Error for ExecError {}

#[derive(FromPrimitive)]
pub enum Instruction {
    Return,
    Call,
    LoadConstant,
    LoadCode,
    MakeFunction,
    LoadGlobal,
    SetGlobal,
    GetAttr,
    SetAttr,
    Pop,
}

pub struct Code {
    upvalues: usize,
    params: usize,
    constants: Vec<Value>,
    instrs: Vec<u8>,
    codes: Vec<Code>,
}

pub fn compile_text<R: Read>(file: R) -> Result<Code, CompileError> {
    // TODO: Compile text into bytecode
    Ok(Code {
        upvalues: 0,
        params: 0,
        constants: vec![Value::String(Rc::new("main".to_owned()))],
        instrs: vec![
            Instruction::LoadCode as u8,
            0, // Code 0 = main
            // Stack: <code>
            Instruction::MakeFunction as u8,
            0, // 0 upvalues
            // Stack: <func main>
            Instruction::SetGlobal as u8, 0, // const 0 = "main"
        ],
        codes: vec![Code {
            upvalues: 0,
            params: 0,
            constants: vec![
                Value::Integer(4),
                Value::String(Rc::new("Child".to_owned())),
                Value::String(Rc::new("greet".to_owned())),
            ],
            instrs: vec![
                Instruction::LoadGlobal as u8,
                1,
                Instruction::LoadConstant as u8,
                0,
                // Stack: <Child> 4
                Instruction::Call as u8,
                1, // 1 argument
                // Stack: c
                Instruction::LoadConstant as u8,
                2, // "greet"
                Instruction::GetAttr as u8,
                // Stack: c <c.greet>
                Instruction::Call as u8,
                0, // 0 arguments
                // Stack: c nil
                Instruction::Pop as u8,
                2,
            ],
            codes: vec![],
        }],
    })
}

pub enum Value {
    String(Rc<String>),
    Integer(i32),
    Nil,
    Code(Rc<Code>),
    Function(Rc<Function>),
    Userdata(usize),
}

struct Function {
    code: Rc<Code>,
}

pub struct VirtualMachine {
    globals: HashMap<String, Value>,
}

impl VirtualMachine {
    pub fn new() -> VirtualMachine {
        VirtualMachine {
            globals: HashMap::new(),
        }
    }

    pub fn load<'a>(&'a mut self, code: Code) -> Thread {
        Thread {
            code: Rc::new(code),
            instr: 0,
            stack: Vec::new(),
        }
    }

    pub fn execute(
        &mut self,
        thread: &mut Thread,
        mut count: Option<usize>,
    ) -> Result<bool, ExecError> {
        while count.unwrap_or(1) > 0 {
            let Thread { code, instr, stack } = thread;
            let code_: &Code = code;
            let Code {
                upvalues,
                params,
                constants,
                instrs,
                codes,
            } = code_;

            // Fetch instruction
            let opcode = if *instr >= instrs.len() {
                // No more instructions, implicit return
                Instruction::Return
            } else {
                let opcode = instrs[*instr];
                *instr += 1;

                // Decode instruction
                match FromPrimitive::from_u8(opcode) {
                    Some(c) => c,
                    None => return Err(ExecError::InvalidInstruction),
                }
            };

            // TODO: Execute instructions
            match opcode {
                Instruction::Return => {}
                Instruction::Call => {}
                Instruction::LoadConstant => {}
                Instruction::LoadCode => {}
                Instruction::MakeFunction => {}
                Instruction::LoadGlobal => {}
                Instruction::SetGlobal => {}
                Instruction::GetAttr => {}
                Instruction::SetAttr => {}
                Instruction::Pop => {}
            }
        }

        Ok(false)
    }
}

pub struct Thread {
    code: Rc<Code>,
    instr: usize,
    stack: Vec<Value>,
}

fn main() {
    let mut vm = VirtualMachine::new();
    let file = match File::open("example.lpc") {
        Ok(f) => f,
        Err(_) => panic!("Couldn't find code"),
    };
    let program = match compile_text(file) {
        Ok(p) => p,
        Err(e) => panic!("Error compiling code: {}", e),
    };
    let mut thread = vm.load(program);
    match vm.execute(&mut thread, None) {
        Ok(true) => {}
        Ok(false) => panic!("Program didn't finish"),
        Err(e) => panic!("Error running program: {}", e),
    }
}
