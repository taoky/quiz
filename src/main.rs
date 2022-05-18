use crossterm::{
    cursor::MoveTo,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
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
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame, Terminal,
};

#[derive(Default, Clone, Debug)]
struct Question {
    description: String,
    options: Option<Vec<(char, String)>>,
}

#[derive(Default, Clone, Debug)]
struct Answer {
    correct_option: Option<char>,
    reason: String,
}

#[derive(Debug)]
enum ParseStateMachine {
    Start,
    ReadQuestionDescription,
    ReadQuestionOptions,
    ReadAnswerCorrectOption,
    ReadAnswerReason,
}

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
            return Err(io::Error::new(io::ErrorKind::Other, "No filename given"));
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
    let mut bank: Vec<(Question, Answer)> = Vec::new();
    for s in contents.split("\n\n") {
        let mut question = Question::default();
        let mut answer = Answer::default();
        let mut state = ParseStateMachine::Start;
        /*
        State machine for each question-answer pair (the order of ParseStateMachine):
        0 -> 1 -> 2 -> 3 -> 4
             |              ^
             |              |
             ----------------
        0 -> 1: Get a question header, start reading question description
        1 -> 2: Get a question-option mark (`===`), start reading question options
        2 -> 3: Get an answer header with question options, start reading answer option
        3 -> 4: After answer's option got, start reading answer reason
        1 -> 4: Get an answer header without question options, start reading answer reason
        */
        for q in s.split('\n') {
            match state {
                ParseStateMachine::Start => match q {
                    "Question" => state = ParseStateMachine::ReadQuestionDescription,
                    _ => panic!(
                        "Wrong format: 'Question' not declared at the top of this question.\n
                                Expected 'Question', found '{}'",
                        q
                    ),
                },
                ParseStateMachine::ReadQuestionDescription => match q {
                    "Answer" => state = ParseStateMachine::ReadAnswerReason,
                    "===" => {
                        question.options = Some(Vec::new());
                        state = ParseStateMachine::ReadQuestionOptions
                    }
                    _ => {
                        question.description.push_str(q);
                        question.description.push('\n');
                    }
                },
                ParseStateMachine::ReadQuestionOptions => match q {
                    "Answer" => state = ParseStateMachine::ReadAnswerCorrectOption,
                    _ => {
                        let mut option = q.split('.');
                        let option_char = option.next().unwrap().chars().next().unwrap();
                        assert!(
                            option_char.is_uppercase(),
                            "Wrong format: question option should be uppercase!"
                        );
                        let option_description = option.collect::<Vec<_>>().join(" ");
                        question
                            .options
                            .as_mut()
                            .unwrap()
                            .push((option_char, option_description));
                    }
                },
                ParseStateMachine::ReadAnswerCorrectOption => {
                    {
                        // check if less than 27 options
                        if question.options.as_ref().unwrap().len() > 26 {
                            panic!("Wrong format: a question with more than 26 options");
                        }
                        // check if duplication
                        let mut uniq = std::collections::HashSet::new();
                        if !question
                            .options
                            .as_ref()
                            .unwrap()
                            .iter()
                            .map(|x| x.0)
                            .all(|x| uniq.insert(x))
                        {
                            panic!("Duplicated options in question options");
                        }
                    }
                    answer.correct_option = Some(q.chars().next().unwrap());
                    // Check consistency
                    let mut checked = false;
                    for (option_char, _) in question.options.as_ref().unwrap() {
                        if option_char == &answer.correct_option.unwrap() {
                            checked = true;
                            break;
                        }
                    }
                    if !checked {
                        panic!("Wrong format: answer's correct option '{}' not found in question's options", answer.correct_option.unwrap());
                    }
                    state = ParseStateMachine::ReadAnswerReason;
                }
                ParseStateMachine::ReadAnswerReason => {
                    answer.reason.push_str(q);
                    answer.reason.push('\n');
                }
            }
        }
        if !matches!(state, ParseStateMachine::ReadAnswerReason) {
            panic!("State error: state machine should be at ReadAnswerReason state at the end of a question.\n
                    Expected 'Answer', found '{:?}'", state);
        }
        if answer.reason.is_empty() && answer.correct_option.is_none() {
            panic!("Wrong format: a non-multiselect question answer's reason should not be empty (question = {:?}).", question);
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

struct UIConfig {
    flip: bool,
    user: Option<char>,
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            flip: false,
            user: None,
        }
    }
}

fn question_internal_shuffle(mut question: Question, mut answer: Answer) -> (Question, Answer) {
    let mut rng = rand::thread_rng();
    if let Some(options) = &mut question.options {
        options.shuffle(&mut rng);
        let alphabet = ('A'..='Z').into_iter().collect::<Vec<char>>();
        let correct_option = answer.correct_option.unwrap();
        for (index, option) in options.iter_mut().enumerate() {
            let old_option = option.0;
            let new_option = alphabet[index];
            if old_option == correct_option {
                answer.correct_option = Some(new_option);
            }
            option.0 = new_option;
        }
    }
    (question, answer)
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    bank: &mut Vec<(Question, Answer)>,
) -> io::Result<()> {
    loop {
        bank.shuffle(&mut rand::thread_rng());

        for (question, answer) in bank.iter() {
            let (question, answer) = question_internal_shuffle(question.clone(), answer.clone());
            let mut config = UIConfig::default();
            loop {
                terminal.draw(|f| ui(f, &question, &answer, &config))?;

                if let Event::Key(key) = event::read()? {
                    // let code = keycode_lower(key.code);
                    if KeyCode::Char('c') == key.code && key.modifiers == KeyModifiers::CONTROL {
                        return Ok(());
                    } else if let KeyCode::Char(' ') = key.code {
                        config.flip = !config.flip;
                        config.user = None;
                    } else if let KeyCode::Enter = key.code {
                        break;
                    } else if let KeyCode::Char(c) = key.code {
                        if !config.flip && question.options.is_some() {
                            // If not in question options, don't change UIConfig
                            let c = c.to_ascii_uppercase();
                            for (option_char, _) in question.options.as_ref().unwrap() {
                                if option_char == &c {
                                    config.user = Some(c);
                                    config.flip = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn question_paragraph(question: &Question) -> Text {
    let mut res = vec![];
    for line in question.description.lines() {
        res.push(Spans::from(line));
    }
    match &question.options {
        Some(options) => {
            for option in options {
                res.push(Spans::from(vec![
                    Span::styled(
                        format!("{}", option.0),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!(".{}", option.1)),
                ]));
            }
        }
        None => {}
    }

    res.into()
}

fn answer_paragraph<'a>(answer: &'a Answer, config: &'a UIConfig) -> Text<'a> {
    /*
        flip false => show instructions only
        flip true => show correct answer (and user's answer if not None)
    */
    if !config.flip {
        return match answer.correct_option {
            Some(_) => Text::raw("Type your answer!"),
            None => Text::raw("This question does not have an option. [SPACE] when ready."),
        };
    }
    let mut res = vec![];
    match &answer.correct_option {
        Some(correct_option) => {
            let mut spans = vec![];
            spans.push(Span::raw(format!("Correct answer: {}.", correct_option)));

            let correct_style = Style::default().bg(Color::Green).fg(Color::White);
            let wrong_style = Style::default().bg(Color::Red).fg(Color::White);

            if let Some(user_option) = config.user {
                spans.push(Span::raw(format!(" Your answer: {}. ", user_option)));
                if *correct_option == user_option {
                    spans.push(Span::styled("Correct!", correct_style));
                } else {
                    spans.push(Span::styled("Wrong!", wrong_style));
                }
            }

            res.push(Spans::from(spans));
        }
        None => {}
    }
    for line in answer.reason.lines() {
        res.push(Spans::from(line));
    }

    res.into()
}

fn ui<B: Backend>(f: &mut Frame<B>, question: &Question, answer: &Answer, config: &UIConfig) {
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
    let paragraph_question = Paragraph::new(question_paragraph(question))
        .block(block_question)
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph_question, chunks[0]);
    let block_answer = Block::default().title("Answer").borders(Borders::ALL);
    let paragraph_answer = Paragraph::new(answer_paragraph(answer, config))
        .block(block_answer)
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph_answer, chunks[1]);
    let paragraph_help =
        Paragraph::new("[SPACE] Show/hide answer, [ENTER] Next question, [Ctrl+C] Quit")
            .block(Block::default());
    f.render_widget(paragraph_help, chunks[2]);
}
