use egg::{define_language, Id, Symbol};
use ordered_float::OrderedFloat;

pub type Num = OrderedFloat<f64>;

define_language! {
    pub enum Calc {
        // values
        Num(Num),
        "true"  = True,
        "false" = False,

        // arithmetic
        "+"   = Add([Id; 2]),
        "-"   = Sub([Id; 2]),
        "*"   = Mul([Id; 2]),
        "/"   = Div([Id; 2]),
        "^"   = Pow([Id; 2]),
        "neg" = Neg([Id; 1]),

        // special functions (single arg)
        "sin"  = Sin([Id; 1]),
        "cos"  = Cos([Id; 1]),
        "tan"  = Tan([Id; 1]),
        "log"  = Log([Id; 1]),
        "ln"   = Ln([Id; 1]),
        "exp"  = Exp([Id; 1]),
        "sqrt" = Sqrt([Id; 1]),
        "abs"  = Abs([Id; 1]),

        // boolean
        "and" = And([Id; 2]),
        "or"  = Or([Id; 2]),
        "xor" = Xor([Id; 2]),
        "not" = Not([Id; 1]),

        // variables / unknown identifiers — must be last
        Symbol(Symbol),
    }
}
