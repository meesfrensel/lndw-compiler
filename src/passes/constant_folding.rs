use std::ops::Neg;

use crate::types::{Expr, Operator};

pub trait ConstantFold {
    fn run_constant_fold(self) -> Self;
}

impl ConstantFold for Expr {
    fn run_constant_fold(self) -> Self {
        match self {
            Expr::Num(_) | Expr::Var(_) =>
            /* no work to be done */
            {
                self
            }
            Expr::UnaryOp(operator, expr) => {
                let e = expr.run_constant_fold();

                if let Expr::Num(n) = e
                    && operator == Operator::Sub
                {
                    return Expr::Num(n.neg());
                }

                Expr::UnaryOp(operator, Box::new(e))
            }
            Expr::BinaryOp(lhs, operator, rhs) => {
                let l = lhs.run_constant_fold();
                let r = rhs.run_constant_fold();

                if let Expr::Num(left) = l
                    && let Expr::Num(right) = r
                {
                    let res = match operator {
                        Operator::Add => left + right,
                        Operator::Sub => left - right,
                        Operator::Mul => left * right,
                        Operator::Div => {
                            if right == 0 {
                                eprintln!("Warning: detected division by zero during constant folding; not folding.");
                                return Expr::BinaryOp(Box::new(l), operator, Box::new(r));
                            }
                            left / right
                        }
                        Operator::Shl => left << right,
                        Operator::Shr => left >> right,
                    };
                    return res.into();
                }

                Expr::BinaryOp(Box::new(l), operator, Box::new(r))
            }
        }
    }
}
