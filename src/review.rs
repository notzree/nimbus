use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::BufRead;
use std::io::Write;
use std::path::PathBuf;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReasonEnum {
    Chatgpt,
    CourseCode,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandEnum {
    Move,
    Skip,
    Indeterminate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub file_path: PathBuf,
    pub command: CommandEnum,
    pub destination: Option<PathBuf>,
    pub reason: Option<ReasonEnum>,
}

const COMMAND_FILE_PATH: &str = "commands.txt";

pub fn write_command(command: Command) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = PathBuf::from(COMMAND_FILE_PATH);
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path)?;
    let serialized_command = serde_json::to_string(&command)?;
    writeln!(file, "{}", serialized_command)?;

    Ok(())
}

pub fn read_commands() -> Result<Vec<Command>, Box<dyn std::error::Error>> {
    let file_path = PathBuf::from(COMMAND_FILE_PATH);
    let mut commands = Vec::new();
    let file = File::open(file_path)?;
    let reader = std::io::BufReader::new(file);
    for lines in reader.lines() {
        let line = lines?;
        match serde_json::from_str::<Command>(&line) {
            Ok(obj) => {
                println!("{:?}", obj);
                commands.push(obj);
            }
            Err(e) => {
                eprintln!("Error deserializing line: {:?}", e);
            }
        }
    }
    Ok(commands)
}
