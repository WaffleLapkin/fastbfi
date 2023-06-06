// Tested* with experimental rustc @ `2ea3dfda03596ecca344ce28de1b804b7b613e48`,
// on `aarch64-unknown-linux-gnu` with LLVM 16.0.4;
//
// *at least the first time...
#![feature(explicit_tail_calls)]

use std::{
    env::args,
    io::{stdin, stdout, Read, Write},
    process::exit,
};

mod compiler;
mod interpreter;
mod lex;

fn main() {
    let mut args = args();
    let binary = args.next().unwrap();
    let Some(path) = args.next() else {
        eprintln!("usage: {binary} <path to a bf program>\n\
                  (input is read from stdin, output is thrown at stdout)\n\
                  (tbqhwy i'm not sure if this is actually fast)\n\
                  (it's maybe probably cool tho)");
        exit(1)
    };

    let source = std::fs::read_to_string(path).unwrap();

    interpret(
        &source,
        &mut || {
            let mut buf = [0];
            stdin().lock().read_exact(&mut buf).unwrap();
            buf[0]
        },
        &mut |c| {
            if c != b'\n' {
                print!("{}", c.escape_ascii());
                stdout().lock().flush().unwrap();
            } else {
                println!();
            }
        },
    )
    .unwrap();
}

/// Interpret a given `source` code of a program and `input`/`output` functions.
///
/// Returns error if the program is malformed (has unmatched `[` and/or `]`).
fn interpret(
    source: &str,
    input: &mut dyn FnMut() -> u8,
    output: &mut dyn FnMut(u8),
) -> Result<(), ()> {
    let lexer = lex::Lexer(source);
    let bc = compiler::compile(lexer)?;

    interpreter::Interpreter(&bc, input, output).run();

    Ok(())
}
