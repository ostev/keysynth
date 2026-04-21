use std::fs;

use bumpalo::Bump;

fn main() {
    repl().unwrap();
}

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
                _ if input.starts_with(":run") => {
                    let file_path = input[":run".len()..].trim();
                    let contents = fs::read_to_string(file_path)?;

                    {
                        let value = calc::run(&ast_arena, &value_arena, &contents);
                        match value {
                            Ok(value) => println!("{:?}", value),
                            Err(error) => println!("{:#?}", error),
                        }
                    }

                    value_arena.reset();
                    ast_arena.reset();
                }
                _ => {
                    {
                        let value = calc::run(&ast_arena, &value_arena, &input);
                        match value {
                            Ok(value) => println!("{:?}", value),
                            Err(error) => println!("{:#?}", error),
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
