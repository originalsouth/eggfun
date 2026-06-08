mod lang;
mod parser;
mod printer;
mod rules;

use std::time::Duration;

use anyhow::Result;
use egg::{AstSize, Extractor, RecExpr, Runner};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::lang::Calc;

const HISTORY_FILE: &str = ".eggfun_history";

const BANNER: &str = "\
eggfun — equation rewriter REPL (egg/equality saturation)
  Numbers, variables, + - * / ^, unary -, sin/cos/tan/log/ln/exp/sqrt/abs
  Boolean: & | xor !, with literals `true` and `false`
  Commands: :help  :rules  :relevant <expr>  :steps <expr>  :quit
";

fn optimize(input: &str) -> Result<(RecExpr<Calc>, RecExpr<Calc>, egg::Report, usize)> {
    let expr = parser::parse(input)?;
    let rules = rules::relevant_rules(&expr);
    let n_rules = rules.len();
    let runner = Runner::default()
        .with_expr(&expr)
        .with_iter_limit(60)
        .with_node_limit(10_000)
        .with_time_limit(Duration::from_secs(2))
        .run(&rules);
    let root = runner.roots[0];
    let extractor = Extractor::new(&runner.egraph, AstSize);
    let (_cost, best) = extractor.find_best(root);
    Ok((expr, best, runner.report(), n_rules))
}

fn run_line(line: &str) {
    let line = line.trim();
    if line.is_empty() { return; }

    if let Some(rest) = line.strip_prefix(':') {
        let (cmd, arg) = match rest.split_once(char::is_whitespace) {
            Some((c, a)) => (c.trim(), a.trim()),
            None => (rest.trim(), ""),
        };
        match cmd {
            "help" | "h" | "?" => println!("{}", BANNER),
            "rules" => println!("rewrite rules (total): {}", rules::rules().len()),
            "relevant" => {
                if arg.is_empty() {
                    eprintln!("usage: :relevant <expr>");
                    return;
                }
                match parser::parse(arg) {
                    Ok(expr) => {
                        let total = rules::rules().len();
                        let n = rules::relevant_rules(&expr).len();
                        println!("relevant rules for `{}`: {} / {}", arg, n, total);
                    }
                    Err(e) => eprintln!("error: {}", e),
                }
            }
            "quit" | "q" | "exit" => std::process::exit(0),
            "steps" => {
                if arg.is_empty() {
                    eprintln!("usage: :steps <expr>");
                    return;
                }
                match optimize(arg) {
                    Ok((orig, best, report, n_rules)) => {
                        println!("input : {}", printer::format(&orig));
                        println!("rules : {} relevant (of {})", n_rules, rules::rules().len());
                        println!("stop  : {:?} after {} iters, {} nodes",
                            report.stop_reason,
                            report.iterations,
                            report.egraph_nodes);
                        println!("best  : {}", printer::format(&best));
                    }
                    Err(e) => eprintln!("error: {}", e),
                }
            }
            other => eprintln!("unknown command `:{}`", other),
        }
        return;
    }

    match optimize(line) {
        Ok((_, best, _, _)) => println!("= {}", printer::format(&best)),
        Err(e) => eprintln!("error: {}", e),
    }
}

fn main() -> Result<()> {
    print!("{}", BANNER);

    let mut rl = DefaultEditor::new()?;
    let hist_path = home_dir().map(|h| h.join(HISTORY_FILE));
    if let Some(p) = &hist_path {
        let _ = rl.load_history(p);
    }

    loop {
        match rl.readline("eggfun> ") {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                run_line(&line);
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(e) => { eprintln!("readline error: {}", e); break; }
        }
    }

    if let Some(p) = &hist_path {
        let _ = rl.save_history(p);
    }
    Ok(())
}

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}
