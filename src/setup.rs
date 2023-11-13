use dirs::download_dir;
use promkit::{error::Result, preset::QuerySelect, preset::Readline};
use serde::Serialize;
use serde_yaml;
use std::error::Error;
use std::fs::File;
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

pub fn setup_nimbus(tx: SyncSender<bool>) -> Result {
    //Check if the config file exists first

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
    let mut coop_prompt = QuerySelect::new([true, false], |text, items| -> Vec<String> {
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
    .title("Do you have Co-op?")
    .item_lines(2)
    .prompt()?;

    config.download_path = PathBuf::from(download_path_prompt.run()?);
    config.base_path = PathBuf::from(base_path_prompt.run()?);
    config.start_year = start_year_prompt.run()?.parse().unwrap();
    config.end_year = end_year_prompt.run()?.parse().unwrap();
    config.coop = coop_prompt.run()?.parse().unwrap();
    write_config(config.clone()).expect("Failed to save config");
    log::info!("Saved config");
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

fn read_config() -> Result<Config> {
    let mut file = File::open("config.yaml")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let parsed_data: Config = serde_yaml::from_str(&contents).unwrap();
    Ok(parsed_data)
}
fn write_config(config: Config) -> Result {
    let yaml_string = serde_yaml::to_string(&config).unwrap();
    // Write the YAML string to a file
    let mut file = File::create("config.yaml")?;
    file.write_all(yaml_string.as_bytes())?;
    Ok(())
}
