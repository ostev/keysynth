use bumpalo::Bump;

fn main() {
    repl().unwrap();
}

fn repl() -> rustyline::Result<()> {
    println!("Use :exit to quit.");
    let mut rl = rustyline::DefaultEditor::new()?;

    loop {
        let readline = rl.readline("> ");
        let bump = Bump::new();
        match readline {
            Ok(input) => match input.as_str() {
                ":exit" => break,
                _ => {
                    let parsed = calc::parser::parse(&bump, &input);
                    println!("{:?}", parsed)
                }
            },
            Err(_) => {}
        }
    }

    Ok(())
}
