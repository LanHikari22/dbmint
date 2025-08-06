use std::path::PathBuf;

use clap::{arg, command, value_parser, Arg, ArgAction, ArgMatches, Command};

fn parse_args() -> ArgMatches {
    command!()
        .arg(
            Arg::new("verbose")
                .help("1: info, 2: debug, 3: trace")
                .short('v')
                .action(ArgAction::Count),
        )
        .subcommand(
            Command::new("gen-lib")
                .about("Generates a Rust library that creates types given the dbml schema provided")
                .arg(
                    arg!([dbml_filename] "dbml file in mount")
                        .required(true)
                        .value_parser(value_parser!(PathBuf)),
                ),
        )
        .subcommand_required(true)
        .get_matches()
}

pub fn main() {
    let matches = parse_args();

    match matches.subcommand() {
        Some(("gen-lib", sub_matches)) => {
            let dbml_filename = sub_matches
                .get_one::<PathBuf>("dbmt_filename")
                .unwrap()
                .to_path_buf();

            println!("OK! We will proceess {dbml_filename:?}");
        }

        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dbml_rs::*;
    use std::fs;

    #[test]
    fn test_hello() {
        assert_eq!(1 + 1, 2);
    }

    #[test]
    fn test_read_ex000_dbml() {
        let input = fs::read_to_string("../../mnt/ex000.dbml").unwrap();
        let result = parse_dbml(&input).unwrap();
    }
}