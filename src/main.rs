use std::env;
use std::fs::read_to_string;

mod astgen;
mod interpreter;
mod tokenizer;
use astgen::ASTGenerator;
use interpreter::Interpreter;
use tokenizer::{Tokenizer, Token};

fn read_file(filename: &str) -> Vec<String> {
    let mut out_lines: Vec<String> = vec![];

    for line in read_to_string(filename).unwrap().lines() {
        out_lines.push(line.to_string())
    }

    return out_lines;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let filepath: Option<&String>;

    if args.len() < 2 {
        eprintln!("Usage: {} [--strict] <filepath>", args[0]);
        std::process::exit(1);
    } else if args.len() == 2 {
        // filepath only
        filepath = Some(&args[1]);
    } else {
        eprintln!("Usage: {} [--strict] <filepath>", args[0]);
        std::process::exit(1);
    }

    let mut tokenizer = Tokenizer::init(read_file(&filepath.unwrap()));
    let mut raw_tokens: Vec<Token> = vec![];
    raw_tokens.push(tokenizer.next_token());

    while raw_tokens.last().unwrap() != &Token::EOF {
        raw_tokens.push(tokenizer.next_token());
    }

    // raw tokens are unusable to the interpreter
    let unraw_tokens = Tokenizer::post_process(raw_tokens);

    for token in &unraw_tokens {
        //println!("{:?}", token);
    }

    let mut astgen = ASTGenerator::init(unraw_tokens);
    astgen.generate_ast();

    for (index, token) in astgen.generated_ast.iter().enumerate() {
        println!("{} | {:?}", index, token);
    }

    let mut interpreter = Interpreter::init(astgen.generated_ast.clone());

    while !interpreter.halted {
        interpreter.execute_one();
        interpreter.print_state();
    }
}
