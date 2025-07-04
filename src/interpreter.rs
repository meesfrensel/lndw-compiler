use rust_i18n::t;
use std::collections::HashMap;
use std::ops::{Add, Div, Mul, Shl, Shr, Sub};

use crate::{
    gui::InterpreterOptions,
    types::{Inst, LpErr, Reg},
};

/// State of the interpreter after executing a single execution step.
pub enum InterpreterState {
    /// Continue execution with the next instruction.
    Continue,
    /// The execution terminated successfully.
    Finished(i32),
}

impl From<i32> for InterpreterState {
    fn from(value: i32) -> Self {
        InterpreterState::Finished(value)
    }
}

/// Interpreter for our custom ISA.
///
/// The interpreters stores the memory layout at each step and thus enables introspection.
pub struct Interpreter {
    /// The register store.
    pub reg_store: HashMap<Reg, i32>,
    /// Slow cache used for out-of-register storage.
    pub ram: Vec<i32>,

    /// Instruction list to be executed.
    instructions: Vec<Inst>,
    /// Program counter pointing to the next instruction to be executed.
    program_counter: usize,

    /// Input variable mapping.
    input_variables: Option<HashMap<String, String>>,

    /// Whether execution has terminated.
    running: bool,

    /// String representation of the current computation
    ///
    /// Needs to be enabled.
    str_repr: String,

    /// Whether string representations should be stored during computation
    repr_enabled: bool,
}

impl Interpreter {
    /// Instantiates a new interpreter with the given hardware configuration.
    pub fn with_config(hw: &InterpreterOptions) -> Self {
        Self {
            reg_store: Default::default(),
            ram: vec![0; hw.num_cachelines],
            instructions: Vec::with_capacity(0),
            str_repr: String::with_capacity(0),
            program_counter: 0,
            input_variables: None,
            running: false,
            repr_enabled: false,
        }
    }

    /// Loads a list of instructions into the interpreter.
    pub fn load_instructions(mut self, instructions: Vec<Inst>) -> Self {
        self.instructions = instructions;
        if self.repr_enabled {
            self.str_repr = self.cur_as_string();
        }
        self
    }

    /// Maps inputs to variables.
    pub fn with_variables(mut self, input_variables: HashMap<String, String>) -> Self {
        self.input_variables = Some(input_variables);
        self
    }

    pub fn with_tracing(mut self) -> Self {
        self.repr_enabled = true;
        if !self.instructions.is_empty() {
            self.str_repr = self.cur_as_string();
        }
        self
    }

    /// Signal to the interpreter that all data has been loaded that was needed.
    pub fn ready(mut self) -> Self {
        self.running = true;
        self
    }

    /// Executes the instruction list until the interpreter either terminates or encounters a critical error.
    pub fn run_to_end(mut self) -> Result<i32, LpErr> {
        loop {
            match self.step()? {
                InterpreterState::Continue => (),
                InterpreterState::Finished(res) => return Ok(res),
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Executes a single step of the program.
    pub fn step(&mut self) -> Result<InterpreterState, LpErr> {
        if !self.running {
            return Err(LpErr::Interpret(
                "The interpreter was either not ready to run or finished execution".into(),
            ));
        }

        if self.program_counter >= self.instructions.len() {
            return Err(LpErr::Interpret("no result found".to_string()));
        }

        if self.repr_enabled {
            self.str_repr = self.cur_as_string();
        }

        match &self.instructions[self.program_counter] {
            Inst::Add(a, b) => run_binop(*a, *b, i32::add, &mut self.reg_store)?,
            Inst::Sub(a, b) => run_binop(*a, *b, i32::sub, &mut self.reg_store)?,
            Inst::Mul(a, b) => run_binop(*a, *b, i32::mul, &mut self.reg_store)?,
            Inst::Div(a, b) => {
                if let Some(0) = self.reg_store.get(b) {
                    return Err(LpErr::Interpret(t!("compiler.error.divzero").to_string()));
                }
                run_binop(*a, *b, i32::div, &mut self.reg_store)?
            }
            Inst::Shl(a, b) => run_binop(*a, *b, i32::shl, &mut self.reg_store)?,
            Inst::Shr(a, b) => run_binop(*a, *b, i32::shr, &mut self.reg_store)?,
            Inst::Store(n, reg) => {
                if self.reg_store.insert(*reg, *n).is_some() {
                    eprintln!("Warning: overwriting register `{reg}`.");
                }
            }
            Inst::Transfer(var, reg) => {
                let vars = self
                    .input_variables
                    .as_ref()
                    .ok_or(LpErr::Interpret("No variables loaded".into()))?;
                if !vars.contains_key(var) {
                    return Err(LpErr::Interpret(t!("compiler.error.unknown_var", v = var).into()));
                }

                let val_str = vars[var].clone();
                if val_str.is_empty() {
                    return Err(LpErr::Interpret(t!("compiler.error.empty_var", v = var).into()));
                }

                let val = val_str.parse::<i32>().map_err(|_| {
                    LpErr::Interpret(t!("compiler.error.nan_var", var = var, val = val_str).into())
                })?;
                if self.reg_store.insert(*reg, val).is_some() {
                    eprintln!("Warning: overwriting register `{reg}`.");
                }
            }
            Inst::Result(r) => {
                self.program_counter += 1;
                self.running = false;
                return Ok((*self
                    .reg_store
                    .get(r)
                    .ok_or(LpErr::Interpret(format!("register `{r}` is empty")))?)
                .into());
            }
            Inst::Write(_, addr) | Inst::Load(addr, _) if addr >= &self.ram.len() => {
                return Err(LpErr::Interpret(format!(
                    "requested RAM address {addr} doesn't exist."
                )));
            }
            Inst::Write(r, addr) => {
                if let Some(val) = self.reg_store.get(r) {
                    self.ram[*addr] = *val;
                } else {
                    return Err(LpErr::Interpret(format!("register `{r}` is empty")));
                }
            }
            Inst::Load(addr, r) => {
                self.reg_store.insert(*r, self.ram[*addr]);
            }
        }

        self.program_counter += 1;
        Ok(InterpreterState::Continue)
    }

    fn cur_as_string(&self) -> String {
        match &self.instructions[self.program_counter] {
            Inst::Add(a, b) => self.display_binop(a, b, "+"),
            Inst::Sub(a, b) => self.display_binop(a, b, "-"),
            Inst::Mul(a, b) => self.display_binop(a, b, "*"),
            Inst::Div(a, b) => self.display_binop(a, b, "/"),
            Inst::Shl(a, b) => self.display_binop(a, b, "<<"),
            Inst::Shr(a, b) => self.display_binop(a, b, ">>"),
            Inst::Store(num, a) => format!("{num} ➡ [{a}]"),
            Inst::Transfer(var, a) => format!("{var} ➡ [{a}]"),
            Inst::Result(a) => format!("= {}", self.reg_store.get(a).unwrap()),
            Inst::Write(reg, addr) => format!("⎘ [{reg}] ➡ [{addr}]"),
            Inst::Load(addr, reg) => format!("⎗ [{reg}] ⬅ [{addr}]"),
        }
    }

    fn display_binop(&self, a: &Reg, b: &Reg, op: &str) -> String {
        format!(
            "{} {op} {}",
            self.reg_store.get(a).unwrap(),
            self.reg_store.get(b).unwrap()
        )
    }

    pub fn display_current(&self) -> &str {
        &self.str_repr
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.program_counter = 0;
        self.ram = self.ram.iter().map(|_| 0).collect();
        self.reg_store.clear();
    }
}

fn run_binop(
    a: Reg,
    b: Reg,
    op: impl FnOnce(i32, i32) -> i32,
    reg_store: &mut HashMap<Reg, i32>,
) -> Result<(), LpErr> {
    match (reg_store.get(&a).cloned(), reg_store.get_mut(&b)) {
        (Some(a), Some(b)) => *b = op(a, *b),
        (None, _) => return Err(LpErr::Interpret(format!("no such reg `{a}`"))),
        (_, None) => return Err(LpErr::Interpret(format!("no such reg `{b}`"))),
    }
    Ok(())
}
