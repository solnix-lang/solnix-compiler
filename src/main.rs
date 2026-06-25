mod ast;
mod compiler;
mod diagnostics;
mod emit;
mod ir;
mod lexer;
mod parser;
mod sema;

use clap::{Arg, ArgMatches, Command};
use compiler::compile;
use std::path::PathBuf;

fn main() {
    let mut app = Command::new("solnixc")
        .version("0.2.0")
        .about("Experimental eBPF compiler for tracepoint programs")
        .subcommand(
            Command::new("compile")
                .about("Compile a Solnix program (.snx) into eBPF object (.o)")
                .arg(
                    Arg::new("input")
                        .help("Input .snx source file")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("output")
                        .help("Output .o file")
                        .required(true)
                        .index(2),
                ),
        );

    let matches = app.clone().get_matches();

    match matches.subcommand() {
        Some(("compile", sub_m)) => compile_cmd(sub_m),
        _ => {
            println!("Solnix Compiler v0.2.0");
            app.print_help().unwrap();
            println!();
        }
    }
}

fn compile_cmd(matches: &ArgMatches) {
    let input_path = PathBuf::from(matches.get_one::<String>("input").unwrap());
    let output_path = PathBuf::from(matches.get_one::<String>("output").unwrap());

    if !input_path.exists() {
        eprintln!("Error: Input file does not exist: {}", input_path.display());
        eprintln!("Please provide a valid .snx source file.");
        std::process::exit(1);
    }

    if let Err(e) = compile(&input_path, &output_path) {
        eprintln!("\nError: {e:?}");
        std::process::exit(1);
    }

    println!("Compilation successful: {}", output_path.display());
}
