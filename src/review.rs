use promkit::preset::Select;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::BufRead;
use std::io::Error;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReasonEnum {
    Chatgpt,
    CourseCode,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommandEnum {
    Move,
    Skip,
    Indeterminate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub file_path: Option<PathBuf>,
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

pub fn read_commands() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = PathBuf::from(COMMAND_FILE_PATH);
    let file = File::open(file_path)?;
    let reader = std::io::BufReader::new(file);
    for lines in reader.lines() {
        let line = lines?;
        match serde_json::from_str::<Command>(&line) {
            Ok(obj) => {
                let mut confirmation_prompt = Select::new(["Y", "N"])
                    .title(format!("Do you want to execute this command?: {:?}", obj))
                    .lines(4)
                    .prompt()?;
                let confirmation_result = confirmation_prompt.run()?;
                if confirmation_result == "Y" {
                    if obj.command == CommandEnum::Move {
                        let file_path = obj.file_path.unwrap();
                        let destination = obj.destination.unwrap();
                        let file_name = get_file_name(file_path.to_str().unwrap()).unwrap();
                        let new_file_path = destination.join(file_name);
                        match std::fs::rename(file_path, new_file_path) {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("Error moving file: {:?}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error deserializing line: {:?}", e);
            }
        }
    }
    clear_file(COMMAND_FILE_PATH)?;

    Ok(())
}

fn get_file_name(path: &str) -> Option<&str> {
    Path::new(path).file_name()?.to_str()
}
fn clear_file(path: &str) -> Result<(), Error> {
    File::create(path)?;
    Ok(())
}
