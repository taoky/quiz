use crossterm::{
    cursor::MoveTo,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use rand::prelude::SliceRandom;
use std::{
    env,
    fs::File,
    io::{self, Read},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};

fn main() -> Result<(), io::Error> {
    // read and parse file
    let args: Vec<_> = env::args().collect();
    let filename: String;
    match args.len() {
        2 => {
            filename = args[1].clone();
        }
        _ => {
            println!("Usage: {} [filename]", args[0]);
            return Err(io::Error::new(io::ErrorKind::Other, "No filename given"))?;
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

    // init terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        Clear(ClearType::All)
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, &mut bank);

    // restroe terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        Clear(ClearType::All),
        MoveTo(0, 0),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    bank: &mut Vec<(String, String)>,
) -> io::Result<()> {
    loop {
        bank.shuffle(&mut rand::thread_rng());

        for (question, answer) in bank.iter() {
            let mut flip = false;
            loop {
                let answer = if flip { answer } else { "" };
                terminal.draw(|f| ui(f, question, answer))?;

                if let Event::Key(key) = event::read()? {
                    if let KeyCode::Char('q') = key.code {
                        return Ok(());
                    } else if let KeyCode::Char(' ') = key.code {
                        flip = !flip;
                    } else if let KeyCode::Enter = key.code {
                        break;
                    }
                }
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, question: &str, answer: &str) {
    let size = f.size();

    let block = Block::default()
        .title("Quiz")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    f.render_widget(block, size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Percentage(49),
                Constraint::Percentage(49),
                Constraint::Percentage(2),
            ]
            .as_ref(),
        )
        .split(size);

    let block_question = Block::default().title("Question").borders(Borders::ALL);
    let paragraph_question = Paragraph::new(question).block(block_question);
    f.render_widget(paragraph_question, chunks[0]);
    let block_answer = Block::default().title("Answer").borders(Borders::ALL);
    let paragraph_answer = Paragraph::new(answer).block(block_answer);
    f.render_widget(paragraph_answer, chunks[1]);
    let paragraph_help =
        Paragraph::new("[SPACE] Show/hide answer, [ENTER] Next question, [q] Quit")
            .block(Block::default());
    f.render_widget(paragraph_help, chunks[2]);
}
