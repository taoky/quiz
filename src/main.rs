#[macro_use(defer)]
extern crate scopeguard;

use nix::sys::termios;
use rand::{self, seq::SliceRandom};
use std::env;
use std::fs::File;
use std::io::prelude::*;

fn refresh() {
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
}

fn wait_enter_refresh() {
    for byte in std::io::stdin().bytes() {
        let byte = byte.unwrap();
        if byte == 10 {
            break;
        }
    }
    refresh();
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let filename: String;
    match args.len() {
        2 => {
            filename = args[1].clone();
        }
        _ => {
            println!("Usage: {} [filename]", args[0]);
            return;
        }
    }
    let mut file = match File::open(&filename) {
        Err(err) => panic!("cannot open {}: {}", filename, err),
        Ok(file) => file,
    };
    let mut contents = String::new();
    if let Err(err) = file.read_to_string(&mut contents) {
        panic!("cannot read {}: {}", filename, err)
    }
    let contents = contents.trim();
    let mut bank: Vec<(String, String)> = Vec::new();
    for s in contents.split("\n\n") {
        let mut question = String::new();
        let mut answer = String::new();
        let mut state = 0;
        for q in s.split('\n') {
            if state == 0 {
                // prepare
                if q == "Question" {
                    state = 1;
                } else {
                    panic!(
                        "Wrong format: 'Question' not declared at the top of this question.\n
                            Expected 'Question', found '{}'",
                        q
                    );
                }
            } else if state == 1 {
                // read question
                if q != "Answer" {
                    question += q;
                    question += "\n";
                } else {
                    state = 2;
                }
            } else {
                // read answer
                answer += q;
                answer += "\n";
            }
        }
        if answer.is_empty() {
            panic!("Answer not found for this question.");
        }
        bank.push((question, answer));
    }

    let orig_term = termios::tcgetattr(0).unwrap();
    defer!(termios::tcsetattr(0, termios::SetArg::TCSADRAIN, &orig_term).unwrap(););
    let mut term = termios::tcgetattr(0).unwrap();
    term.local_flags.remove(termios::LocalFlags::ICANON); // get chars immediately
    term.local_flags.remove(termios::LocalFlags::ECHO); // noecho
    refresh();
    loop {
        bank.shuffle(&mut rand::thread_rng());
        for (question, answer) in &bank {
            print!("问题:\n{}", question);
            wait_enter_refresh();
            print!("答案:\n{}", answer);
            wait_enter_refresh();
        }
    }
}
