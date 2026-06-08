//! Pretty-print a `RecExpr<Calc>` back to infix syntax with minimal parens.

use egg::{Id, RecExpr};

use crate::lang::Calc;

/// Operator precedence (higher = binds tighter). Atoms get u8::MAX.
fn prec(node: &Calc) -> u8 {
    match node {
        Calc::Or(_)  => 1,
        Calc::Xor(_) => 2,
        Calc::And(_) => 3,
        Calc::Not(_) => 4,
        Calc::Add(_) | Calc::Sub(_) => 5,
        Calc::Mul(_) | Calc::Div(_) => 6,
        Calc::Neg(_) => 7,
        Calc::Pow(_) => 8,
        _ => u8::MAX,
    }
}

fn is_right_assoc(node: &Calc) -> bool { matches!(node, Calc::Pow(_)) }

pub fn format(expr: &RecExpr<Calc>) -> String {
    let mut out = String::new();
    let root = Id::from(expr.as_ref().len() - 1);
    write_node(expr, root, 0, &mut out);
    out
}

fn write_node(expr: &RecExpr<Calc>, id: Id, parent_prec: u8, out: &mut String) {
    let node = &expr[id];
    let my_prec = prec(node);
    let needs_parens = my_prec < parent_prec;
    if needs_parens { out.push('('); }
    write_bare(expr, node, out);
    if needs_parens { out.push(')'); }
    let _ = my_prec; // silence
}

fn write_bare(expr: &RecExpr<Calc>, node: &Calc, out: &mut String) {
    use std::fmt::Write;
    match node {
        Calc::Num(n) => {
            let v = n.into_inner();
            if v == v.trunc() && v.abs() < 1e16 {
                let _ = write!(out, "{}", v as i64);
            } else {
                let _ = write!(out, "{}", v);
            }
        }
        Calc::True  => out.push_str("true"),
        Calc::False => out.push_str("false"),
        Calc::Symbol(s) => out.push_str(s.as_str()),

        Calc::Add([a, b]) => binop(expr, *a, *b, " + ", node, out),
        Calc::Sub([a, b]) => binop(expr, *a, *b, " - ", node, out),
        Calc::Mul([a, b]) => binop(expr, *a, *b, " * ", node, out),
        Calc::Div([a, b]) => binop(expr, *a, *b, " / ", node, out),
        Calc::Pow([a, b]) => binop(expr, *a, *b, "^",   node, out),

        Calc::And([a, b]) => binop(expr, *a, *b, " & ", node, out),
        Calc::Or([a, b])  => binop(expr, *a, *b, " | ", node, out),
        Calc::Xor([a, b]) => binop(expr, *a, *b, " xor ", node, out),

        Calc::Neg([a]) => { out.push('-'); write_node(expr, *a, prec(node), out); }
        Calc::Not([a]) => { out.push('!'); write_node(expr, *a, prec(node), out); }

        Calc::Sin([a])  => call(expr, "sin",  *a, out),
        Calc::Cos([a])  => call(expr, "cos",  *a, out),
        Calc::Tan([a])  => call(expr, "tan",  *a, out),
        Calc::Log([a])  => call(expr, "log",  *a, out),
        Calc::Ln([a])   => call(expr, "ln",   *a, out),
        Calc::Exp([a])  => call(expr, "exp",  *a, out),
        Calc::Sqrt([a]) => call(expr, "sqrt", *a, out),
        Calc::Abs([a])  => call(expr, "abs",  *a, out),
    }
}

fn binop(expr: &RecExpr<Calc>, a: Id, b: Id, sep: &str, parent: &Calc, out: &mut String) {
    let p = prec(parent);
    // For left-assoc: left allows equal prec, right needs strictly greater.
    // For right-assoc (Pow): swap.
    let (lp, rp) = if is_right_assoc(parent) { (p + 1, p) } else { (p, p + 1) };
    write_node(expr, a, lp, out);
    out.push_str(sep);
    write_node(expr, b, rp, out);
}

fn call(expr: &RecExpr<Calc>, name: &str, a: Id, out: &mut String) {
    out.push_str(name);
    out.push('(');
    write_node(expr, a, 0, out);
    out.push(')');
}
