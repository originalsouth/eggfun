`cargo run --release`
```
    Finished `release` profile [optimized] target(s) in 0.01s
     Running `target/release/eggfun`
eggfun — equation rewriter REPL (egg/equality saturation)
  Numbers, variables, + - * / ^, unary -, sin/cos/tan/log/ln/exp/sqrt/abs
  Boolean: & | xor !, with literals `true` and `false`
  Commands: :help  :rules  :relevant <expr>  :steps <expr>  :quit
eggfun> sin(exp(ln(sqrt(x^2)))+1)^2+cos((abs(x)+1)^2/(abs(x)+1))^2
= 1
eggfun>
```
