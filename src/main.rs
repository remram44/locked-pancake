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
    codes: Vec<Rc<Code>>,
}

pub struct Function {
    code: Rc<Code>,
    upvalues: Vec<Value>,
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
        codes: vec![Rc::new(Code {
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
        })],
    })
}

#[derive(Clone)]
pub enum Value {
    String(Rc<String>),
    Integer(i32),
    Nil,
    Code(Rc<Code>),
    Function(Rc<Function>),
    Userdata(usize),
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

            // Execute instructions
            match opcode {
                Instruction::Return => match (stack.pop(), stack.pop()) {
                    (Some(Value::Integer(i)), Some(Value::Code(c)))
                        if i >= 0 =>
                    {
                        *instr = i as usize;
                        *code = c;
                    }
                    (Some(_), Some(_)) => {
                        return Err(ExecError::InvalidInstruction);
                    }
                    _ => {
                        return Err(ExecError::StackEmpty);
                    }
                },
                Instruction::Call => {
                    // Function call needs the function and the arguments to be
                    // on the stack, and pushes the current instruction counter
                    // and code object before switching to the new code

                    // Read operand: number of arguments on stack
                    let nb_args = instrs[*instr] as usize;
                    *instr += 1;

                    // Check stack
                    if stack.len() < nb_args + 1 {
                        return Err(ExecError::StackEmpty);
                    }

                    // Get the function object
                    let func = match &stack[stack.len() - 1 - nb_args] {
                        Value::Function(f) => f.clone(),
                        _ => return Err(ExecError::InvalidInstruction),
                    };
                    let func_code: &Code = &func.code;

                    if func_code.params > nb_args {
                        // Set missing arguments to nil
                        stack.reserve(func_code.params - nb_args);
                        for _ in nb_args..func_code.params {
                            stack.push(Value::Nil);
                        }
                    } else if func_code.params < nb_args {
                        // Remove extra arguments
                        stack.truncate(
                            stack.len() + func_code.params - nb_args,
                        );
                    }

                    if func_code.upvalues > 0 {
                        // TODO: Deal with upvalues somehow
                        return Err(ExecError::InvalidInstruction);
                    }

                    // Push the previous instruction counter and code object
                    stack.push(Value::Integer(*instr as i32));
                    stack.push(Value::Code(code.clone()));

                    // Switch to the new code
                    *instr = 0;
                    *code = func.code.clone();
                }
                Instruction::LoadConstant => {
                    // Read operand: constant number
                    let constant_idx = instrs[*instr] as usize;
                    *instr += 1;

                    // Get constant value
                    let value = if constant_idx < code.constants.len() {
                        code.constants[constant_idx].clone()
                    } else {
                        return Err(ExecError::InvalidInstruction);
                    };

                    // Put it on the stack
                    stack.push(value);
                }
                Instruction::LoadCode => {
                    // Read operand: code number
                    let code_idx = instrs[*instr] as usize;
                    *instr += 1;

                    // Get code
                    let code_obj = if code_idx < code.codes.len() {
                        code.codes[code_idx].clone()
                    } else {
                        return Err(ExecError::InvalidInstruction);
                    };

                    // Put it on the stack
                    stack.push(Value::Code(code_obj));
                }
                Instruction::MakeFunction => {
                    // Read operand: number of upvalues
                    let nb_upvalues = instrs[*instr] as usize;
                    *instr += 1;

                    if nb_upvalues > 0 {
                        // TODO: Implement upvalues
                        return Err(ExecError::InvalidInstruction);
                    }

                    // Check stack
                    if stack.len() < nb_upvalues + 1 {
                        return Err(ExecError::StackEmpty);
                    }

                    // Get the upvalues
                    let func_upvalues =
                        stack.split_off(stack.len() - nb_upvalues);

                    // Get the code object
                    let code_obj = match stack.pop() {
                        Some(Value::Code(c)) => c,
                        _ => return Err(ExecError::InvalidInstruction),
                    };

                    // Make the function object on the stack
                    let func = Rc::new(Function {
                        code: code_obj,
                        upvalues: func_upvalues,
                    });
                    stack.push(Value::Function(func));
                }
                Instruction::LoadGlobal => {}
                Instruction::SetGlobal => {}
                Instruction::GetAttr => {}
                Instruction::SetAttr => {}
                Instruction::Pop => {
                    // Read operand: number of values to pop from stack
                    let nb = instrs[*instr] as usize;
                    *instr += 1;

                    // Check stack
                    if stack.len() < nb {
                        return Err(ExecError::StackEmpty);
                    }

                    // Pop
                    stack.truncate(stack.len() - nb);
                }
            }

            match count {
                Some(ref mut c) => *c -= 1,
                None => {}
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
