use std::collections::HashSet;

use egg::{
    rewrite as rw, Analysis, Applier, ENodeOrVar, EGraph, Id, PatternAst, RecExpr, Rewrite, Subst,
    Var,
};
use ordered_float::OrderedFloat;

use crate::lang::Calc;

pub type Rules = Vec<Rewrite<Calc, ()>>;

fn is_const(var: &str) -> impl Fn(&mut EGraph<Calc, ()>, Id, &Subst) -> bool {
    let v: Var = var.parse().unwrap();
    move |egraph, _, subst| {
        egraph[subst[v]].nodes.iter().any(|n| matches!(n, Calc::Num(_)))
    }
}

fn is_not_zero(var: &str) -> impl Fn(&mut EGraph<Calc, ()>, Id, &Subst) -> bool {
    let v: Var = var.parse().unwrap();
    move |egraph, _, subst| {
        egraph[subst[v]].nodes.iter().all(|n| match n {
            Calc::Num(OrderedFloat(x)) => *x != 0.0,
            _ => true,
        })
    }
}

/// Folds binary arithmetic when both operands are concrete numbers.
struct ConstFold {
    op: &'static str,
    a: Var,
    b: Var,
}

impl Applier<Calc, ()> for ConstFold {
    fn apply_one(
        &self,
        egraph: &mut EGraph<Calc, ()>,
        eclass: Id,
        subst: &Subst,
        _searcher_ast: Option<&PatternAst<Calc>>,
        _rule_name: egg::Symbol,
    ) -> Vec<Id> {
        let a = num_of(egraph, subst[self.a]);
        let b = num_of(egraph, subst[self.b]);
        let (Some(a), Some(b)) = (a, b) else { return vec![] };
        let r = match self.op {
            "+" => a + b,
            "-" => a - b,
            "*" => a * b,
            "/" => if b == 0.0 { return vec![] } else { a / b },
            "^" => a.powf(b),
            _ => return vec![],
        };
        if !r.is_finite() { return vec![] }
        let new_id = egraph.add(Calc::Num(OrderedFloat(r)));
        if egraph.union(eclass, new_id) { vec![eclass] } else { vec![] }
    }
}

struct UnaryFold {
    op: &'static str,
    a: Var,
}

impl Applier<Calc, ()> for UnaryFold {
    fn apply_one(
        &self,
        egraph: &mut EGraph<Calc, ()>,
        eclass: Id,
        subst: &Subst,
        _searcher_ast: Option<&PatternAst<Calc>>,
        _rule_name: egg::Symbol,
    ) -> Vec<Id> {
        let Some(a) = num_of(egraph, subst[self.a]) else { return vec![] };
        let r = match self.op {
            "neg"  => -a,
            "abs"  => a.abs(),
            "sqrt" => if a < 0.0 { return vec![] } else { a.sqrt() },
            "exp"  => a.exp(),
            "ln"   => if a <= 0.0 { return vec![] } else { a.ln() },
            _ => return vec![],
        };
        if !r.is_finite() { return vec![] }
        let new_id = egraph.add(Calc::Num(OrderedFloat(r)));
        if egraph.union(eclass, new_id) { vec![eclass] } else { vec![] }
    }
}

fn num_of<A: Analysis<Calc>>(egraph: &EGraph<Calc, A>, id: Id) -> Option<f64> {
    egraph[id].nodes.iter().find_map(|n| match n {
        Calc::Num(OrderedFloat(x)) => Some(*x),
        _ => None,
    })
}

pub fn rules() -> Rules {
    let mut r: Rules = vec![
        // ── commutativity ──
        rw!("comm-add"; "(+ ?a ?b)" => "(+ ?b ?a)"),
        rw!("comm-mul"; "(* ?a ?b)" => "(* ?b ?a)"),
        rw!("comm-and"; "(and ?a ?b)" => "(and ?b ?a)"),
        rw!("comm-or";  "(or ?a ?b)"  => "(or ?b ?a)"),
        rw!("comm-xor"; "(xor ?a ?b)" => "(xor ?b ?a)"),

        // ── associativity ──
        rw!("assoc-add"; "(+ (+ ?a ?b) ?c)" => "(+ ?a (+ ?b ?c))"),
        rw!("assoc-mul"; "(* (* ?a ?b) ?c)" => "(* ?a (* ?b ?c))"),
        rw!("assoc-and"; "(and (and ?a ?b) ?c)" => "(and ?a (and ?b ?c))"),
        rw!("assoc-or";  "(or (or ?a ?b) ?c)"   => "(or ?a (or ?b ?c))"),
        rw!("assoc-xor"; "(xor (xor ?a ?b) ?c)" => "(xor ?a (xor ?b ?c))"),

        // ── arithmetic identities ──
        rw!("add-zero";  "(+ ?a 0)" => "?a"),
        rw!("mul-one";   "(* ?a 1)" => "?a"),
        rw!("mul-zero";  "(* ?a 0)" => "0"),
        rw!("sub-self";  "(- ?a ?a)" => "0"),
        rw!("sub-to-neg"; "(- ?a ?b)" => "(+ ?a (neg ?b))"),
        rw!("zero-sub";  "(- 0 ?a)" => "(neg ?a)"),
        rw!("neg-add";   "(+ ?a (neg ?a))" => "0"),
        rw!("neg-neg";   "(neg (neg ?a))" => "?a"),
        rw!("neg-zero";  "(neg 0)" => "0"),
        rw!("neg-distrib-add"; "(neg (+ ?a ?b))" => "(+ (neg ?a) (neg ?b))"),
        rw!("neg-mul-l"; "(* (neg ?a) ?b)" => "(neg (* ?a ?b))"),
        rw!("neg-mul-rev"; "(neg (* ?a ?b))" => "(* (neg ?a) ?b)"),
        rw!("pow-zero";  "(^ ?a 0)" => "1"),
        rw!("pow-one";   "(^ ?a 1)" => "?a"),
        rw!("one-pow";   "(^ 1 ?a)" => "1"),
        rw!("zero-pow";  "(^ 0 ?a)" => "0" if is_not_zero("?a")),
        rw!("pow-pow";   "(^ (^ ?a ?m) ?n)" => "(^ ?a (* ?m ?n))"),
        rw!("pow-mul-distrib"; "(^ (* ?a ?b) ?n)" => "(* (^ ?a ?n) (^ ?b ?n))"),
        rw!("div-self";  "(/ ?a ?a)" => "1" if is_not_zero("?a")),
        rw!("div-one";   "(/ ?a 1)" => "?a"),
        rw!("div-zero";  "(/ 0 ?a)" => "0" if is_not_zero("?a")),
        rw!("div-mul-cancel"; "(* (/ ?a ?b) ?b)" => "?a" if is_not_zero("?b")),

        // ── combine like terms ──
        rw!("mul-collect-a"; "(+ ?a ?a)" => "(* 2 ?a)"),
        rw!("coeff-collect-1"; "(+ ?a (* ?n ?a))" => "(* (+ ?n 1) ?a)"),
        rw!("mul-square";    "(* ?a ?a)" => "(^ ?a 2)"),
        rw!("pow-add";       "(* (^ ?a ?m) (^ ?a ?n))" => "(^ ?a (+ ?m ?n))"),
        rw!("pow-one-times"; "(* ?a (^ ?a ?n))" => "(^ ?a (+ ?n 1))"),
        rw!("pow-div";       "(/ (^ ?a ?n) ?a)" => "(^ ?a (- ?n 1))" if is_not_zero("?a")),
        rw!("pow-div-pow";   "(/ (^ ?a ?m) (^ ?a ?n))" => "(^ ?a (- ?m ?n))" if is_not_zero("?a")),
        rw!("mul-div-l";     "(/ (* ?a ?b) ?a)" => "?b" if is_not_zero("?a")),
        rw!("mul-div-r";     "(/ (* ?a ?b) ?b)" => "?a" if is_not_zero("?b")),

        // ── distribution (both directions) ──
        rw!("distrib";   "(* ?a (+ ?b ?c))" => "(+ (* ?a ?b) (* ?a ?c))"),
        rw!("factor";    "(+ (* ?a ?b) (* ?a ?c))" => "(* ?a (+ ?b ?c))"),

        // ── special function identities ──
        rw!("sin-zero";  "(sin 0)"  => "0"),
        rw!("cos-zero";  "(cos 0)"  => "1"),
        rw!("tan-zero";  "(tan 0)"  => "0"),
        rw!("sin-neg";   "(sin (neg ?a))" => "(neg (sin ?a))"),
        rw!("cos-neg";   "(cos (neg ?a))" => "(cos ?a)"),
        rw!("tan-neg";   "(tan (neg ?a))" => "(neg (tan ?a))"),
        rw!("exp-zero";  "(exp 0)"  => "1"),
        rw!("exp-add";   "(* (exp ?a) (exp ?b))" => "(exp (+ ?a ?b))"),
        rw!("exp-pow";   "(^ (exp ?a) ?b)" => "(exp (* ?a ?b))"),
        rw!("ln-one";    "(ln 1)"   => "0"),
        rw!("ln-exp";    "(ln (exp ?a))" => "?a"),
        rw!("exp-ln";    "(exp (ln ?a))" => "?a"),
        rw!("ln-mul";    "(ln (* ?a ?b))" => "(+ (ln ?a) (ln ?b))"),
        rw!("ln-pow";    "(ln (^ ?a ?b))" => "(* ?b (ln ?a))"),
        rw!("ln-div";    "(ln (/ ?a ?b))" => "(- (ln ?a) (ln ?b))"),
        rw!("log-div";   "(log (/ ?a ?b))" => "(- (log ?a) (log ?b))"),
        rw!("sqrt-one";  "(sqrt 1)" => "1"),
        rw!("sqrt-zero"; "(sqrt 0)" => "0"),
        rw!("sqrt-mul";  "(* (sqrt ?a) (sqrt ?a))" => "?a"),
        rw!("sqrt-sq";   "(sqrt (^ ?a 2))" => "(abs ?a)"),
        rw!("abs-abs";   "(abs (abs ?a))" => "(abs ?a)"),
        rw!("abs-neg";   "(abs (neg ?a))" => "(abs ?a)"),
        rw!("abs-mul";   "(abs (* ?a ?b))" => "(* (abs ?a) (abs ?b))"),
        rw!("abs-sq";    "(^ (abs ?a) 2)" => "(^ ?a 2)"),
        rw!("pythag";    "(+ (^ (sin ?a) 2) (^ (cos ?a) 2))" => "1"),
        rw!("log-mul";   "(log (* ?a ?b))" => "(+ (log ?a) (log ?b))"),
        rw!("log-pow";   "(log (^ ?a ?b))" => "(* ?b (log ?a))"),

        // ── boolean identities ──
        rw!("and-self";    "(and ?a ?a)"   => "?a"),
        rw!("or-self";     "(or ?a ?a)"    => "?a"),
        rw!("and-true";    "(and ?a true)" => "?a"),
        rw!("and-false";   "(and ?a false)" => "false"),
        rw!("or-false";    "(or ?a false)" => "?a"),
        rw!("or-true";     "(or ?a true)"  => "true"),
        rw!("and-not";     "(and ?a (not ?a))" => "false"),
        rw!("or-not";      "(or ?a (not ?a))"  => "true"),
        rw!("not-not";     "(not (not ?a))" => "?a"),
        rw!("not-true";    "(not true)"  => "false"),
        rw!("not-false";   "(not false)" => "true"),
        rw!("demorgan-and"; "(not (and ?a ?b))" => "(or (not ?a) (not ?b))"),
        rw!("demorgan-or";  "(not (or ?a ?b))"  => "(and (not ?a) (not ?b))"),
        rw!("absorb-and";  "(and ?a (or ?a ?b))" => "?a"),
        rw!("absorb-or";   "(or ?a (and ?a ?b))" => "?a"),
        rw!("xor-self";    "(xor ?a ?a)" => "false"),
        rw!("xor-false";   "(xor ?a false)" => "?a"),
        rw!("xor-true";    "(xor ?a true)" => "(not ?a)"),
    ];

    // ── constant folding (binary) ──
    let a: Var = "?a".parse().unwrap();
    let b: Var = "?b".parse().unwrap();
    for op in ["+", "-", "*", "/", "^"] {
        let pat = format!("({} ?a ?b)", op);
        r.push(
            Rewrite::new(
                format!("fold-{}", op),
                pat.parse::<egg::Pattern<Calc>>().unwrap(),
                ConstFold { op, a, b },
            )
            .unwrap(),
        );
    }

    // ── constant folding (unary) ──
    for op in ["neg", "abs", "sqrt", "exp", "ln"] {
        let pat = format!("({} ?a)", op);
        r.push(
            Rewrite::new(
                format!("fold-{}", op),
                pat.parse::<egg::Pattern<Calc>>().unwrap(),
                UnaryFold { op, a },
            )
            .unwrap(),
        );
    }

    // Conditional rules referencing helpers (build separately for borrow clarity).
    r
}

#[allow(dead_code)]
fn _silence_unused_helper() {
    let _ = is_const("?x");
}

// ────────────────────────────────────────────────────────────────────────────
// Per-expression rule pruning
//
// Tag each rule with the set of `Calc` variants appearing in its LHS / RHS
// patterns, then for a given input compute the transitively-reachable variant
// set and keep only rules whose LHS variants are all reachable. This is a
// sound under-approximation: a rule whose LHS uses operators not in the
// e-graph cannot fire.
// ────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
enum Tag {
    Num, True, False, Symbol,
    Add, Sub, Mul, Div, Pow, Neg,
    Sin, Cos, Tan, Log, Ln, Exp, Sqrt, Abs,
    And, Or, Xor, Not,
}

fn tag(c: &Calc) -> Tag {
    match c {
        Calc::Num(_)    => Tag::Num,
        Calc::True      => Tag::True,
        Calc::False     => Tag::False,
        Calc::Symbol(_) => Tag::Symbol,
        Calc::Add(_)    => Tag::Add,
        Calc::Sub(_)    => Tag::Sub,
        Calc::Mul(_)    => Tag::Mul,
        Calc::Div(_)    => Tag::Div,
        Calc::Pow(_)    => Tag::Pow,
        Calc::Neg(_)    => Tag::Neg,
        Calc::Sin(_)    => Tag::Sin,
        Calc::Cos(_)    => Tag::Cos,
        Calc::Tan(_)    => Tag::Tan,
        Calc::Log(_)    => Tag::Log,
        Calc::Ln(_)     => Tag::Ln,
        Calc::Exp(_)    => Tag::Exp,
        Calc::Sqrt(_)   => Tag::Sqrt,
        Calc::Abs(_)    => Tag::Abs,
        Calc::And(_)    => Tag::And,
        Calc::Or(_)     => Tag::Or,
        Calc::Xor(_)    => Tag::Xor,
        Calc::Not(_)    => Tag::Not,
    }
}

fn pattern_tags(pat: &PatternAst<Calc>) -> HashSet<Tag> {
    pat.as_ref()
        .iter()
        .filter_map(|n| match n {
            ENodeOrVar::ENode(c) => Some(tag(c)),
            ENodeOrVar::Var(_) => None,
        })
        .collect()
}

fn expr_tags(e: &RecExpr<Calc>) -> HashSet<Tag> {
    e.as_ref().iter().map(tag).collect()
}

fn rule_tags(r: &Rewrite<Calc, ()>) -> (HashSet<Tag>, HashSet<Tag>) {
    let lhs = r
        .searcher
        .get_pattern_ast()
        .map(pattern_tags)
        .unwrap_or_default();

    // Dynamic-applier fold rules don't expose a pattern; they always produce a Num.
    let rhs = if r.name.as_str().starts_with("fold-") {
        let mut s = HashSet::new();
        s.insert(Tag::Num);
        s
    } else {
        r.applier
            .get_pattern_ast()
            .map(pattern_tags)
            .unwrap_or_default()
    };

    (lhs, rhs)
}

/// Return the subset of `rules()` whose LHS operators can match somewhere
/// reachable from `expr`. Sound (never prunes a rule that could fire).
pub fn relevant_rules(expr: &RecExpr<Calc>) -> Rules {
    let annotated: Vec<(Rewrite<Calc, ()>, HashSet<Tag>, HashSet<Tag>)> = rules()
        .into_iter()
        .map(|r| {
            let (l, rh) = rule_tags(&r);
            (r, l, rh)
        })
        .collect();

    let mut reachable = expr_tags(expr);
    loop {
        let mut changed = false;
        for (_, lhs, rhs) in &annotated {
            if lhs.is_subset(&reachable) {
                for t in rhs {
                    if reachable.insert(*t) {
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }

    annotated
        .into_iter()
        .filter_map(|(r, lhs, _)| {
            if lhs.is_subset(&reachable) {
                Some(r)
            } else {
                None
            }
        })
        .collect()
}
