use chumsky::prelude::*;
use std::collections::{HashMap, HashSet};
use std::ops::{Add, Div, Mul, Sub};
use std::vec;

pub use crate::types::*;

#[derive(Debug, PartialEq, Clone)]
enum Statement {
    Expression(Expr),
    Assignment { name: String, value: Expr },
}

#[derive(Debug, PartialEq, Clone)]
struct Program {
    statements: Vec<Statement>,
}

fn expression_parser<'src>(
) -> impl Parser<'src, &'src str, Expr, extra::Err<Rich<'src, char>>> + Clone {
    recursive(|expr| {
        let number = text::int(10).from_str().unwrapped().map(Expr::Num).padded();
        let ident = text::ident().padded();
        let variable = ident.map(|s: &str| Expr::Var(s.to_string()));

        let atom = choice((
            number,
            variable,
            expr.clone().delimited_by(just('('), just(')')).padded(),
        ));

        let unary = just('-')
            .padded()
            .repeated()
            .foldr(atom.clone(), |_op, rhs| {
                Expr::UnaryOp(Operator::Sub, Box::new(rhs))
            });

        let op = |c: char| just(c).padded();

        let factor = unary.clone().foldl(
            choice((op('*').to(Operator::Mul), op('/').to(Operator::Div)))
                .then(unary)
                .repeated(),
            |lhs, (op, rhs)| Expr::BinaryOp(Box::new(lhs), op, Box::new(rhs)),
        );

        factor.clone().foldl(
            choice((op('+').to(Operator::Add), op('-').to(Operator::Sub)))
                .then(factor)
                .repeated(),
            |lhs, (op, rhs)| Expr::BinaryOp(Box::new(lhs), op, Box::new(rhs)),
        )
    })
}

fn statement_parser<'src>() -> impl Parser<'src, &'src str, Statement, extra::Err<Rich<'src, char>>>
{
    let expr = expression_parser();
    let ident = text::ident().padded();

    let assignment = ident
        .then_ignore(just('=').padded())
        .then(expr.clone())
        .map(|(name, value)| Statement::Assignment {
            name: name.to_string(),
            value,
        });

    let expression = expr.map(Statement::Expression);

    choice((assignment, expression))
}

fn program_parser<'src>() -> impl Parser<'src, &'src str, Program, extra::Err<Rich<'src, char>>> {
    let statement = statement_parser();

    statement
        .padded()
        .separated_by(just(';').padded())
        .allow_trailing()
        .collect::<Vec<_>>()
        .map(|statements| Program { statements })
}

fn fold_constants(expr: Expr) -> Result<Expr, LpErr> {
    match expr {
        Expr::BinaryOp(left, op, right) => {
            let left = fold_constants(*left)?;
            let right = fold_constants(*right)?;

            if op == Operator::Div {
                if let Expr::Num(0) = right {
                    return Err(LpErr::Compile("Division by zero detected".to_string()));
                }
            }

            match (&left, &op, &right) {
                (Expr::Num(0), Operator::Add, _) => Ok(right),
                (_, Operator::Add, Expr::Num(0)) => Ok(left),
                (_, Operator::Sub, Expr::Num(0)) => Ok(left),
                (Expr::Num(0), Operator::Mul, _) | (_, Operator::Mul, Expr::Num(0)) => {
                    Ok(Expr::Num(0))
                }
                (Expr::Num(1), Operator::Mul, _) => Ok(right),
                (_, Operator::Mul, Expr::Num(1)) => Ok(left),
                (_, Operator::Div, Expr::Num(1)) => Ok(left),
                (Expr::Num(a), Operator::Add, Expr::Num(b)) => Ok(Expr::Num(a + b)),
                (Expr::Num(a), Operator::Sub, Expr::Num(b)) => Ok(Expr::Num(a - b)),
                (Expr::Num(a), Operator::Mul, Expr::Num(b)) => Ok(Expr::Num(a * b)),
                (Expr::Num(a), Operator::Div, Expr::Num(b)) => {
                    if *b == 0 {
                        Err(LpErr::Compile("Division by zero detected".to_string()))
                    } else {
                        Ok(Expr::Num(a / b))
                    }
                }
                _ => Ok(Expr::BinaryOp(Box::new(left), op, Box::new(right))),
            }
        }
        Expr::UnaryOp(op, expr) => {
            let expr = fold_constants(*expr)?;
            match (&op, &expr) {
                (Operator::Sub, Expr::UnaryOp(Operator::Sub, inner)) => Ok((**inner).clone()),
                (Operator::Sub, Expr::Num(n)) => Ok(Expr::Num(-n)),
                _ => Ok(Expr::UnaryOp(op, Box::new(expr))),
            }
        }
        _ => Ok(expr),
    }
}

fn extract_factors(left: &Expr, right: &Expr) -> Vec<Expr> {
    let left_factors = get_multiplication_factors(left);
    let right_factors = get_multiplication_factors(right);

    let mut common_factors = Vec::new();

    for left_factor in &left_factors {
        for right_factor in &right_factors {
            if expressions_equal(left_factor, right_factor) {
                common_factors.push(left_factor.clone());
            }
        }
    }

    common_factors
}

fn get_multiplication_factors(expr: &Expr) -> Vec<Expr> {
    match expr {
        Expr::BinaryOp(left, Operator::Mul, right) => {
            let mut factors = get_multiplication_factors(left);
            factors.extend(get_multiplication_factors(right));
            factors
        }
        _ => vec![expr.clone()],
    }
}

fn expressions_equal(a: &Expr, b: &Expr) -> bool {
    match (a, b) {
        (Expr::Num(n1), Expr::Num(n2)) => n1 == n2,
        (Expr::Var(v1), Expr::Var(v2)) => v1 == v2,
        (Expr::UnaryOp(op1, e1), Expr::UnaryOp(op2, e2)) => op1 == op2 && expressions_equal(e1, e2),
        (Expr::BinaryOp(l1, op1, r1), Expr::BinaryOp(l2, op2, r2)) => {
            op1 == op2 && expressions_equal(l1, l2) && expressions_equal(r1, r2)
        }
        _ => false,
    }
}

fn remove_factor_from_expr(expr: &Expr, factor: &Expr) -> Expr {
    match expr {
        Expr::BinaryOp(left, Operator::Mul, right) => {
            if expressions_equal(left, factor) {
                (**right).clone()
            } else if expressions_equal(right, factor) {
                (**left).clone()
            } else {
                let new_left = remove_factor_from_expr(left, factor);
                let new_right = remove_factor_from_expr(right, factor);

                if expressions_equal(&new_left, left) && expressions_equal(&new_right, right) {
                    expr.clone()
                } else {
                    Expr::BinaryOp(Box::new(new_left), Operator::Mul, Box::new(new_right))
                }
            }
        }
        _ => {
            if expressions_equal(expr, factor) {
                Expr::Num(1)
            } else {
                expr.clone()
            }
        }
    }
}

fn extract_common_factors(expr: Expr) -> Result<Expr, LpErr> {
    match expr {
        Expr::BinaryOp(left, Operator::Add, right) => {
            let left = extract_common_factors(*left)?;
            let right = extract_common_factors(*right)?;

            let common_factors = extract_factors(&left, &right);

            if !common_factors.is_empty() {
                let factor = &common_factors[0];
                let left_remainder = remove_factor_from_expr(&left, factor);
                let right_remainder = remove_factor_from_expr(&right, factor);

                let sum = Expr::BinaryOp(
                    Box::new(left_remainder),
                    Operator::Add,
                    Box::new(right_remainder),
                );

                Ok(Expr::BinaryOp(
                    Box::new(factor.clone()),
                    Operator::Mul,
                    Box::new(sum),
                ))
            } else {
                Ok(Expr::BinaryOp(
                    Box::new(left),
                    Operator::Add,
                    Box::new(right),
                ))
            }
        }
        Expr::BinaryOp(left, op, right) => {
            let left = extract_common_factors(*left)?;
            let right = extract_common_factors(*right)?;
            Ok(Expr::BinaryOp(Box::new(left), op, Box::new(right)))
        }
        Expr::UnaryOp(op, expr) => {
            let expr = extract_common_factors(*expr)?;
            Ok(Expr::UnaryOp(op, Box::new(expr)))
        }
        _ => Ok(expr),
    }
}

fn collect_used_variables(expr: &Expr, used: &mut HashSet<String>) {
    match expr {
        Expr::Var(name) => {
            used.insert(name.clone());
        }
        Expr::BinaryOp(left, _, right) => {
            collect_used_variables(left, used);
            collect_used_variables(right, used);
        }
        Expr::UnaryOp(_, expr) => {
            collect_used_variables(expr, used);
        }
        Expr::Num(_) => {}
    }
}

fn eliminate_dead_code(program: &Program) -> Program {
    let mut used_variables = HashSet::new();
    let mut result_statements = Vec::new();

    let mut last_expr = None;
    for statement in &program.statements {
        match statement {
            Statement::Expression(expr) => {
                last_expr = Some(expr);
            }
            Statement::Assignment { .. } => {}
        }
    }

    if let Some(expr) = last_expr {
        collect_used_variables(expr, &mut used_variables);
    }

    let mut changed = true;
    while changed {
        changed = false;
        for statement in &program.statements {
            if let Statement::Assignment { name, value } = statement {
                if used_variables.contains(name) {
                    let old_size = used_variables.len();
                    collect_used_variables(value, &mut used_variables);
                    if used_variables.len() > old_size {
                        changed = true;
                    }
                }
            }
        }
    }

    for statement in &program.statements {
        match statement {
            Statement::Expression(_) => {
                result_statements.push(statement.clone());
            }
            Statement::Assignment { name, .. } => {
                if used_variables.contains(name) {
                    result_statements.push(statement.clone());
                }
            }
        }
    }

    Program {
        statements: result_statements,
    }
}

fn ast_to_ir(
    ast: &Expr,
    next_reg: &mut u8,
    code: &mut Vec<Inst>,
    variables: &mut HashSet<String>,
    var_registers: &HashMap<String, u8>,
) -> Result<u8, LpErr> {
    match ast {
        Expr::Num(n) => {
            let reg = *next_reg;
            code.push(Inst::Store(*n, u8tochar(reg)));
            *next_reg += 1;
            Ok(reg)
        }
        Expr::Var(v) => {
            if let Some(&reg) = var_registers.get(v) {
                Ok(reg)
            } else {
                let reg = *next_reg;
                code.push(Inst::Transfer(v.clone(), u8tochar(reg)));
                variables.insert(v.clone());
                *next_reg += 1;
                Ok(reg)
            }
        }
        Expr::UnaryOp(Operator::Sub, e) => {
            let left_reg = ast_to_ir(&Expr::Num(0), next_reg, code, variables, var_registers)?;
            let right_reg = ast_to_ir(e, next_reg, code, variables, var_registers)?;
            code.push(Inst::Sub(u8tochar(left_reg), u8tochar(right_reg)));
            Ok(right_reg)
        }
        Expr::UnaryOp(op, _) => Err(LpErr::IR(format!("invalid unary operator `{op}`"))),
        Expr::BinaryOp(left, op, right) => {
            let left_reg = ast_to_ir(left, next_reg, code, variables, var_registers)?;
            let right_reg = ast_to_ir(right, next_reg, code, variables, var_registers)?;

            let inst = match op {
                Operator::Add => Inst::Add(u8tochar(left_reg), u8tochar(right_reg)),
                Operator::Sub => Inst::Sub(u8tochar(left_reg), u8tochar(right_reg)),
                Operator::Mul => Inst::Mul(u8tochar(left_reg), u8tochar(right_reg)),
                Operator::Div => Inst::Div(u8tochar(left_reg), u8tochar(right_reg)),
            };

            code.push(inst);
            Ok(right_reg)
        }
    }
}

fn generate_ir(
    program: &Program,
    do_dead_code_elimination: bool,
) -> Result<(Vec<Inst>, HashSet<String>), LpErr> {
    let optimized_program = if do_dead_code_elimination {
        eliminate_dead_code(program)
    } else {
        program.clone()
    };

    let mut reg_counter = 0;
    let mut code: Vec<Inst> = vec![];
    let mut variables = HashSet::new();
    let mut var_registers = HashMap::new();
    let mut last_reg = None;

    for statement in &optimized_program.statements {
        match statement {
            Statement::Expression(expr) => {
                last_reg = Some(ast_to_ir(
                    expr,
                    &mut reg_counter,
                    &mut code,
                    &mut variables,
                    &var_registers,
                )?);
            }
            Statement::Assignment { name, value } => {
                let value_reg = ast_to_ir(
                    value,
                    &mut reg_counter,
                    &mut code,
                    &mut variables,
                    &var_registers,
                )?;
                var_registers.insert(name.clone(), value_reg);
                last_reg = Some(value_reg);
            }
        }
    }

    if let Some(reg) = last_reg {
        code.push(Inst::Result(u8tochar(reg)));
    } else {
        return Err(LpErr::IR("No expression to evaluate".to_string()));
    }

    Ok((code, variables))
}

fn check_division_by_zero(expr: &Expr) -> Result<(), LpErr> {
    match expr {
        Expr::BinaryOp(_, Operator::Div, right) => {
            if let Expr::Num(0) = **right {
                return Err(LpErr::Compile("Division by zero detected".to_string()));
            }
            check_division_by_zero(right)?;
        }
        Expr::BinaryOp(left, _, right) => {
            check_division_by_zero(left)?;
            check_division_by_zero(right)?;
        }
        Expr::UnaryOp(_, expr) => {
            check_division_by_zero(expr)?;
        }
        _ => {}
    }
    Ok(())
}

pub fn compile(
    input: &String,
    do_constant_folding: bool,
    do_dead_code_elimination: bool,
    do_common_factor_extraction: bool,
) -> Result<((Vec<Inst>, HashSet<String>), (Vec<Inst>, HashSet<String>)), LpErr> {
    let program = program_parser()
        .parse(input.trim())
        .into_result()
        .map_err(|e| LpErr::Parse(format!("Parse error: {:?}", e)))?;

    // Generate unoptimized version first
    let unoptimized_result = generate_ir(&program, false)?;

    // Apply optimizations
    let mut optimized_program = program.clone();
    for statement in &mut optimized_program.statements {
        match statement {
            Statement::Expression(expr) => {
                if do_constant_folding {
                    *expr = fold_constants(expr.clone())?;
                }
                if do_common_factor_extraction {
                    *expr = extract_common_factors(expr.clone())?;
                }
                check_division_by_zero(expr)?;
            }
            Statement::Assignment { value, .. } => {
                if do_constant_folding {
                    *value = fold_constants(value.clone())?;
                }
                if do_common_factor_extraction {
                    *value = extract_common_factors(value.clone())?;
                }
                check_division_by_zero(value)?;
            }
        }
    }

    let optimized_result = generate_ir(&optimized_program, do_dead_code_elimination)?;

    Ok((unoptimized_result, optimized_result))
}

fn u8tochar(reg: u8) -> char {
    char::from_digit(reg as u32 + 10, 36).unwrap()
}

pub fn interpret_ir(
    instructions: Vec<Inst>,
    input_variables: &HashMap<String, String>,
) -> Result<i32, LpErr> {
    let mut reg_store = HashMap::<Reg, i32>::new();

    for inst in instructions {
        println!("Variable store is: {reg_store:?}");
        match inst {
            Inst::Add(a, b) => run_binop(a, b, i32::add, &mut reg_store)?,
            Inst::Sub(a, b) => run_binop(a, b, i32::sub, &mut reg_store)?,
            Inst::Mul(a, b) => run_binop(a, b, i32::mul, &mut reg_store)?,
            Inst::Div(a, b) => {
                let b_val = check_store_contains(&reg_store, b)?;
                if b_val == 0 {
                    return Err(LpErr::Runtime("Runtime division by zero".to_string()));
                }
                run_binop(a, b, i32::div, &mut reg_store)?
            }
            Inst::Store(n, reg) => {
                if reg_store.contains_key(&reg) {
                    eprintln!("Warning: overwriting register `{reg}`.");
                    reg_store.get_mut(&reg).map(|v| *v = n);
                } else {
                    reg_store.insert(reg, n);
                }
            }
            Inst::Transfer(v, _) if !input_variables.contains_key(&v) => {
                return Err(LpErr::Interpret(format!("unknown variable `{v}`")))
            }
            Inst::Transfer(_, r) if reg_store.contains_key(&r) => {
                return Err(LpErr::Interpret(format!(
                    "register `{r}` already contains value"
                )))
            }
            Inst::Transfer(var, reg) => {
                let val_str = input_variables[&var].clone();
                let val = val_str
                    .parse::<i32>()
                    .map_err(|_| LpErr::Interpret(format!("`{val_str}` is not a number")))?;
                reg_store.insert(reg, val);
            }
            Inst::Result(r) => {
                return Ok(*reg_store
                    .get(&r)
                    .ok_or(LpErr::Interpret(format!("register `{}` is empty", r)))?)
            }
        }
    }
    Err(LpErr::Interpret("no result found".to_string()))
}

fn run_binop(
    a: Reg,
    b: Reg,
    op: impl FnOnce(i32, i32) -> i32,
    reg_store: &mut HashMap<Reg, i32>,
) -> Result<(), LpErr> {
    let a = check_store_contains(reg_store, a)?;
    check_store_contains(reg_store, b)?;
    reg_store.get_mut(&b).map(|b| *b = op(a, *b));
    Ok(())
}

fn check_store_contains(store: &HashMap<Reg, i32>, key: Reg) -> Result<i32, LpErr> {
    match store.get(&key) {
        Some(v) => Ok(*v),
        None => Err(LpErr::Interpret(format!("no such reg `{}`", key))),
    }
}
