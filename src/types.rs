use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum LpErr {
    SExpr(String),
    Parse(String),
    IR(String),
    Interpret(String),
    Compile(String),
    Runtime(String),
}

impl Display for LpErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LpErr::SExpr(e) => write!(f, "{} (s-expr)", e),
            LpErr::Parse(e) => write!(f, "{} (parse)", e),
            LpErr::IR(e) => write!(f, "{} (ir gen)", e),
            LpErr::Interpret(e) => write!(f, "{} (interpreter)", e),
            LpErr::Compile(e) => write!(f, "{} (compile)", e),
            LpErr::Runtime(e) => write!(f, "{} (runtime)", e),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token {
    Open,
    Close,
    Sym(String),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SExpr {
    Sym(String),
    List(Vec<SExpr>),
}

impl Display for SExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SExpr::Sym(e) => write!(f, "{}", e),
            SExpr::List(es) => {
                write!(f, "(")?;
                for (count, v) in es.iter().enumerate() {
                    if count != 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, ")")
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
}

impl Display for Operator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Operator::Add => write!(f, "+"),
            Operator::Sub => write!(f, "-"),
            Operator::Mul => write!(f, "*"),
            Operator::Div => write!(f, "/"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Expr {
    Num(i32),
    Var(String),
    UnaryOp(Operator, Box<Expr>),
    BinaryOp(Box<Expr>, Operator, Box<Expr>),
}

pub type Reg = char;

#[derive(Debug, Clone)]
pub enum Inst {
    Add(Reg, Reg),
    Sub(Reg, Reg),
    Mul(Reg, Reg),
    Div(Reg, Reg),
    Store(i32, Reg),
    Transfer(String, Reg),
    Result(Reg),
}

impl Display for Inst {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Inst::Add(a, b) => write!(f, "add register {a} to register {b}"),
            Inst::Sub(a, b) => write!(f, "subtract register {a} from register {b}"),
            Inst::Mul(a, b) => write!(f, "multiply register {a} by register {b}"),
            Inst::Div(a, b) => write!(f, "divide register {a} by register {b}"),
            Inst::Store(n, r) => write!(f, "store the number {n} in register {r}"),
            Inst::Transfer(v, r) => write!(f, "transfer variable {v} to register {r}"),
            Inst::Result(r) => write!(f, "the result is in register {r}"),
        }
    }
}
