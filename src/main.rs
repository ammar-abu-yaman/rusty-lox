use std::env;
use std::fs;
use std::io::{self, Write};
use std::process::exit;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        writeln!(io::stderr(), "Usage: {} tokenize <filename>", args[0]).unwrap();
        return;
    }

    let command = &args[1];
    let filename = &args[2];

    match command.as_str() {
        "tokenize" => {
            let file_contents = fs::read_to_string(filename).unwrap_or_else(|_| {
                writeln!(io::stderr(), "Failed to read file {}", filename).unwrap();
                String::new()
            });
            
            let mut line: u64 = 1;
            let mut has_errors = false;
            file_contents.chars().for_each(|c| match c {
                '(' => println!("LEFT_PAREN ( null"),
                ')' => println!("RIGHT_PAREN ) null"),
                '{' => println!("LEFT_BRACE {{ null"),
                '}' => println!("RIGHT_BRACE }} null"),
                '+' => println!("PLUS + null"),
                '-' => println!("MINUS - null"),
                '.' => println!("DOT . null"),
                '*' => println!("STAR * null"),
                ',' => println!("COMMA , null"),
                ';' => println!("SEMICOLON ; null"),
                ' ' | '\t' | '\r' => {},
                '\n' => line += 1,
                unexpected => {
                    eprintln!("[line {line}] Error: Unexpected character: {unexpected}");
                    has_errors = true;
                }
            });
            println!("EOF  null");
            if has_errors { 
                exit(65);
            }
        }
        _ => {
            writeln!(io::stderr(), "Unknown command: {}", command).unwrap();
            return;
        }
    }
}
