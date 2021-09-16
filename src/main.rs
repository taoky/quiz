use pancurses::{endwin, initscr, Input, noecho};
use rand::{self, seq::SliceRandom};
use std::env;
use std::fs::File;
use std::io::prelude::*;
#[macro_use(defer)] extern crate scopeguard;

fn wait_enter_refresh(window: &pancurses::Window) {
    loop {
        match window.getch() {
            Some(Input::Character('\n')) => { break }
            Some(Input::KeyEnter) => { break }
            _ => ()
        }
    }
    window.clear();
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

    let window = initscr();
    defer!(endwin(););
    noecho();
    loop {
        bank.shuffle(&mut rand::thread_rng());
        for (question, answer) in &bank {
            window.printw("问题:\n".to_string() + question);
            wait_enter_refresh(&window);
            window.printw("答案:\n".to_string() + answer);
            wait_enter_refresh(&window);
        }
    }
}
