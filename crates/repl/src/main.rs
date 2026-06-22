use std::fs;

use bumpalo::Bump;
use calc::{ExecutionError, interpreter};
use codespan_reporting::{
    diagnostic::Diagnostic,
    files::SimpleFile,
    term::{self, WriteStyle},
};

fn main() {
    repl().unwrap();
}

/// A simple test REPL for the calculator language.
fn repl() -> rustyline::Result<()> {
    println!("Use :exit to quit.");
    let mut rl = rustyline::DefaultEditor::new()?;

    loop {
        let readline = rl.readline("> ");
        let mut ast_arena = Bump::new();
        let mut value_arena = Bump::new();
        match readline {
            Ok(input) => match input.as_str() {
                ":exit" => break,
                _ => {
                    {
                        match calc::parser::parse(&ast_arena, &input) {
                            Ok(expr) => match calc::interpreter::eval_root(&expr) {
                                Ok(value) => println!("{}", value),
                                Err(error) => {
                                    let diagnostic: Diagnostic<()> = error.into();
                                    let text = term::emit_into_string(
                                        &term::Config::default(),
                                        &SimpleFile::new("repl", &input),
                                        &diagnostic,
                                    )
                                    .unwrap();
                                    println!("{}", text);
                                }
                            },
                            Err(error) => {
                                let diagnostic: Diagnostic<()> = error.into();
                                let text = term::emit_into_string(
                                    &term::Config::default(),
                                    &SimpleFile::new("repl", &input),
                                    &diagnostic,
                                )
                                .unwrap();
                                println!("{}", text);
                            }
                        }
                    }

                    value_arena.reset();
                    ast_arena.reset();
                }
            },
            Err(_) => {}
        }
    }

    Ok(())
}
