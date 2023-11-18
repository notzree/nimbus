use dirs::download_dir;
use promkit::{preset::QuerySelect, preset::Readline, preset::Select};
use serde::Serialize;
use serde_yaml;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, SyncSender};

#[derive(Default, Serialize, serde::Deserialize, Debug, Clone)]
pub struct Config {
    pub download_path: PathBuf,
    pub base_path: PathBuf,
    pub start_year: i32,
    pub end_year: i32,
    pub coop: bool,
}

pub fn setup_nimbus(tx: SyncSender<bool>) -> Result<(), Box<dyn Error>> {
    if Path::new("config.yaml").exists() {
        log::info!("Config file exists");
        let mut continue_prompt = QuerySelect::new(['Y', 'N'], |text, items| -> Vec<String> {
            text.parse::<usize>()
                .map(|query| {
                    items
                        .iter()
                        .filter(|num| query <= num.parse::<usize>().unwrap_or_default())
                        .map(|num| num.to_string())
                        .collect::<Vec<String>>()
                })
                .unwrap_or(items.clone())
        })
        .title("Config file exists. Continuing will overwrite the existing config file. Proceed?")
        .item_lines(2)
        .prompt()?;
        if continue_prompt.run()? == 'N'.to_string() {
            log::info!("Exiting...");
            std::process::exit(0);
        }
    }
    let config = parse_user_input()?;
    write_config(config.clone()).expect("Failed to save config");
    log::info!("Saved config");

    let mut term_prompt = Select::new(["1A", "1B", "2A", "2B", "3A", "3B", "4A", "4B"])
        .title("What term are you currently in?")
        .lines(4)
        .prompt()?;
    let current_term = term_prompt.run()?;
    match create_term_directories(&current_term, config.clone().base_path) {
        Ok(_) => log::info!("Created term directories"),
        Err(e) => log::error!("Failed to create term directories: {}", e),
    }

    let mut daemon_prompt = QuerySelect::new([true, false], |text, items| -> Vec<String> {
        text.parse::<usize>()
            .map(|query| {
                items
                    .iter()
                    .filter(|num| query <= num.parse::<usize>().unwrap_or_default())
                    .map(|num| num.to_string())
                    .collect::<Vec<String>>()
            })
            .unwrap_or(items.clone())
    })
    .title("Start daemon?")
    .item_lines(2)
    .prompt()?;
    let daemon = daemon_prompt.run()?.parse().unwrap();
    tx.send(daemon).unwrap();
    Ok(())
}

fn parse_user_input() -> Result<Config, Box<dyn Error>> {
    let mut config = Config::default();
    let mut download_path_prompt = Readline::default()
        .title("where is the directory for your downloads")
        .validator(
            |text| Path::new(text).is_dir(),
            |text| format!("Must be a valid directory. Got {} instead", text),
        )
        .prompt()?;

    let mut base_path_prompt = Readline::default()
        .title("where is your base directory for your files")
        .validator(
            |text| Path::new(text).is_dir(),
            |text| format!("Must be a valid directory. Got {} instead", text),
        )
        .prompt()?;

    let mut start_year_prompt = Readline::default()
        .title("When do you start university")
        .validator(
            |text| text.parse::<i32>().is_ok(),
            |text| format!("Must be a valid number. Got {} instead", text),
        )
        .prompt()?;

    let mut end_year_prompt = Readline::default()
        .title("When do you end university")
        .validator(
            |text| text.parse::<i32>().is_ok(),
            |text| format!("Must be a valid number. Got {} instead", text),
        )
        .prompt()?;

    let mut coop_prompt = Select::new([true, false])
        .title("Do you have co-op?")
        .lines(2)
        .prompt()?;

    config.download_path = PathBuf::from(download_path_prompt.run()?);
    config.base_path = PathBuf::from(base_path_prompt.run()?);
    config.start_year = start_year_prompt.run()?.parse().unwrap();
    config.end_year = end_year_prompt.run()?.parse().unwrap();
    config.coop = coop_prompt.run()?.parse().unwrap();
    Ok(config)
}

fn read_config() -> Result<Config, io::Error> {
    let mut file = File::open("config.yaml")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let parsed_data: Config = serde_yaml::from_str(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(parsed_data)
}
fn write_config(config: Config) -> Result<(), io::Error> {
    let yaml_string =
        serde_yaml::to_string(&config).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let mut file = File::create("config.yaml")?;
    file.write_all(yaml_string.as_bytes())?;
    Ok(())
}

fn create_term_directories(current_term: &str, base_dir: PathBuf) -> Result<(), io::Error> {
    let end_term_num = 4;
    let end_term_char = 'B';

    let curr_term_num = current_term
        .chars()
        .nth(0)
        .and_then(|c| c.to_digit(10))
        .ok_or(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid term number",
        ))?;
    let curr_term_char = current_term.chars().nth(1).ok_or(io::Error::new(
        io::ErrorKind::InvalidInput,
        "Invalid term character",
    ))?;

    for i in curr_term_num..=end_term_num {
        for j in curr_term_char as u8..=end_term_char as u8 {
            let term = format!("{}{}", i, j as char);
            let term_path = base_dir.join(term);
            if !term_path.exists() {
                fs::create_dir(term_path)?;
            }
        }
    }
    Ok(())
}
