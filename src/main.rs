use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;
use async_recursion::async_recursion;
use rand::seq::SliceRandom;

const MIN_WORD_LENGTH: i32 = 3;
const MAX_WORD_LENGTH: i32 = 25;
const STEPS: i32 = 7;

struct GameManager {
    word: Vec<Letter>,
    already_guessed: Vec<char>,
    steps_left: i32,
}

impl GameManager {
    fn new() -> GameManager {
        GameManager {
            word: Vec::new(),
            already_guessed: Vec::new(),
            steps_left: STEPS,
        }
    }

    async fn start_game(&mut self) {
        loop {
            self.flush();
            let word = self.choose_word().await;
            self.init_word(&word).await;

            self.play().await;

            let mut play_again = String::new();
            print!("Do you want to play again? (Y/N) ");
            io::stdout().flush().unwrap();
            io::stdin().read_line(&mut play_again).unwrap();
            if play_again.trim().to_lowercase() != "y" {
                println!("Goodbye!");
                break;
            }
        }
    }

    #[async_recursion]
    async fn choose_word(&self) -> String {
        print!("Do you want to use the default dictionary? (Y/N) ");
        io::stdout().flush().unwrap();

        let mut dictionary_or_not = String::new();
        io::stdin().read_line(&mut dictionary_or_not).unwrap();

        match dictionary_or_not.trim().to_lowercase().as_str() {
            "y" | "" => self.load_default_dictionary().await.choose(&mut rand::thread_rng()).unwrap().clone(),
            "n" => {
                loop {
                    print!("Enter a word: ");
                    io::stdout().flush().unwrap();
                    let mut word = String::new();
                    io::stdin().read_line(&mut word).unwrap();
                    let word = word.trim();
                    let mut is_valid = true;

                    for i in word.chars() {
                        if !i.is_alphabetic() || i.to_string().len() != 1 {
                            is_valid = false;
                            break
                        }
                    }

                    if word.len() < (MIN_WORD_LENGTH as usize) || word.len() > (MAX_WORD_LENGTH as usize) {
                        println!("Word must be between {} and {} characters long", MIN_WORD_LENGTH, MAX_WORD_LENGTH);
                    } else if !is_valid {
                        println!("Word must not contain special chars or symbols.")
                    } else {
                        break String::from(word);
                    }
                }
            }
            _ => {
                println!("Please enter y or n");
                self.choose_word().await
            }
        }
    }

    async fn load_default_dictionary(&self) -> Vec<String> {
        let file = File::open("dictionary.txt").unwrap();
        io::BufReader::new(file).lines().map(|line| line.unwrap()).collect()
    }

    async fn init_word(&mut self, word: &str) {
        self.word.clear();
        for letter in word.chars() {
            self.word.push(Letter { letter, status: letter == ' ' });
        }
    }

    async fn print_word(&self, end: bool) -> String {
        if end {
            self.word.iter().map(|letter| letter.letter).collect()
        } else {
            self.word.iter().map(|letter| if letter.status { letter.letter } else { '_' }).collect()
        }
    }

    async fn check_letter(&mut self, letter: char) -> bool {
        let mut found = false;
        for i in 0..self.word.len() {
            if self.word[i].letter.to_lowercase().next().unwrap() == letter.to_ascii_lowercase() {
                self.word[i].status = true;
                found = true;
                self.already_guessed.push(letter.to_lowercase().next().unwrap());
            }
        }
        found
    }

    async fn check_win(&self) -> bool {
        self.word.iter().all(|letter| letter.status)
    }

    async fn check_lose(&self) -> bool {
        self.steps_left == 0
    }

    async fn print_status(&self, end: bool) {
        println!("{}", generate_frame(self.steps_left as usize, self.print_word(end).await).await);
    }

    async fn play(&mut self) {
        loop {
            self.print_status(false).await;
            let letter = self.ask("Enter a letter: ").await.chars().next().unwrap();
            if letter.is_alphabetic() && letter.to_string().len() == 1 {
                if self.already_guessed.contains(&letter.to_lowercase().next().unwrap()) {
                    println!("You already guessed this letter");
                } else {
                    if !self.check_letter(letter).await {
                        self.already_guessed.push(letter.to_lowercase().next().unwrap());
                        self.steps_left -= 1;
                    }
                    if self.check_win().await {
                        self.print_status(true).await;
                        println!("You won!");
                        break;
                    }
                    if self.check_lose().await {
                        self.print_status(true).await;
                        println!("You lost!");
                        break;
                    }
                }
            } else {
                println!("Please enter only one letter.");
            }
        }
    }

    fn flush(&mut self) {
        self.steps_left = STEPS;
        self.word.clear();
        self.already_guessed.clear();
    }

    async fn ask(&self, question: &str) -> String {
        print!("{}", question);
        io::stdout().flush().unwrap();
        let mut answer = String::new();
        io::stdin().read_line(&mut answer).unwrap();
        answer.trim().to_string()
    }
}

struct Letter {
    letter: char,
    status: bool,
}

async fn generate_frame(steps: usize, word: String) -> String {
    let frames = read_frames_from_file("frames.txt").await.unwrap_or_else(|_| {
        println!("Failed to read frames from file.");
        std::process::exit(1);
    });

    if frames.is_empty() {
        println!("No frames available.");
        std::process::exit(1)
    }

    let header = "########## Hangman ##########";
    let footer = format!("########## {} steps ##########", steps);
    let word_frame = format!("# {}{}{} #", " ".repeat((25 - word.len()) / 2), word, " ".repeat((25 - word.len()) / 2 + (25 - word.len()) % 2));
    let frame_index = if steps > 7 { 0 } else { 7 - steps };

    format!("{}\n{}\n{}\n{}", header, frames[frame_index].join("\n"), word_frame, footer)
}

async fn read_frames_from_file<P: AsRef<Path>>(file_path: P) -> io::Result<Vec<Vec<String>>> {
    let file = File::open(file_path)?;
    let lines = io::BufReader::new(file).lines();

    let mut frames = Vec::new();
    let mut current_frame = Vec::new();

    for line in lines {
        let line = line?;
        if line.starts_with('-') {
            if !current_frame.is_empty() {
                frames.push(current_frame.clone());
                current_frame.clear();
            }
        } else {
            current_frame.push(line);
        }
    }

    if !current_frame.is_empty() {
        frames.push(current_frame);
    }

    Ok(frames)
}

#[tokio::main]
async fn main() {
    let mut game_manager = GameManager::new();
    game_manager.start_game().await;
}
