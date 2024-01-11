use crate::review::{write_command, Command, CommandEnum, ReasonEnum};

use crate::setup::{read_config, Config, Course};
use chatgpt::prelude::*;
use notify::event::{CreateKind, EventKind, ModifyKind};
use notify::{Error, Event, FsEventWatcher};
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
use plist::Value;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::{
    path::Path,
    sync::mpsc::{channel, Sender},
    time::Duration,
};
use tokio::*;

#[derive(Debug, Clone)]
pub struct File {
    name: String,
    url: String,
    path: PathBuf,
}

enum ParseEventResult {
    File(File),
    Empty,
}
enum GrepCourseCodeResult {
    Course(Course),
    Empty,
}

pub fn start_monitor() -> std::result::Result<(), Box<dyn std::error::Error>> {
    log::info!("Starting monitor...");
    let gpt_client =
        ChatGPT::new(std::env::var("GPT_API_KEY").expect("OpenAI API key required")).unwrap();
    let mut conversation: Conversation = gpt_client.new_conversation_directed(
        "You are a LLM designed to categorize downloaded files into their  ",
    );
    let (action_tx, mut action_rx) = tokio::sync::mpsc::unbounded_channel();
    let config = read_config()?;
    let directory_map = create_directory_map(&config)?;
    let courses = config.courses;
    let download_path = config.download_path;
    log::info!("Starting monitor on {}", download_path.to_str().unwrap());
    let (debouncer_tx, debouncer_rx) = channel();

    let _debouncer =
        create_debouncer(download_path, debouncer_tx.clone()).expect("Failed to create debouncer");

    tokio::spawn(async move {
        while let Some(event) = action_rx.recv().await {
            let file = parse_event(event);
            if let Ok(ParseEventResult::File(file)) = file {
                log::info!("File: {:?}", file);
                match grep_course_code(&file, courses.clone()) {
                    Ok(GrepCourseCodeResult::Course(_course)) => {
                        let directory = directory_map.get(&_course.name).unwrap();
                        let command: Command = Command {
                            file_path: Some(file.path.clone()),
                            command: CommandEnum::Move,
                            destination: Some(directory.to_path_buf()), // TODO: Add destination.
                            reason: Some(ReasonEnum::CourseCode),
                        };
                        log::info!("Saving command: {:?}", command);
                        write_command(command).unwrap();
                    }
                    Ok(GrepCourseCodeResult::Empty) => {
                        let contents = get_file_contents(file.path.to_str().unwrap());
                        match contents {
                            Ok(contents) => {}
                            Err(e) => {
                                let command: Command = Command {
                                    file_path: (None),
                                    command: (CommandEnum::Indeterminate),
                                    destination: (None),
                                    reason: (None),
                                };
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Error: {}", e);
                    }
                }
            } else if let Err(e) = file {
                log::error!("Error: {}", e);
            }
        }
    });

    for result in debouncer_rx {
        match result {
            Ok(events) => events
                .iter()
                .for_each(|event| action_tx.send(event.clone()).unwrap()),
            Err(errors) => errors.iter().for_each(|error| log::error!("{error:?}")),
        }
    }

    Ok(())
}

fn create_debouncer<P: AsRef<Path>>(
    path: P,
    tx: Sender<std::result::Result<Vec<DebouncedEvent>, Vec<Error>>>,
) -> notify::Result<(Debouncer<FsEventWatcher, FileIdMap>, ())> {
    let mut debouncer = new_debouncer(Duration::from_secs(1), None, tx)?;
    debouncer
        .watcher()
        .watch(path.as_ref(), RecursiveMode::Recursive)?;
    debouncer
        .cache()
        .add_root(path.as_ref(), RecursiveMode::Recursive);

    Ok((debouncer, ()))
}

fn create_directory_map(
    config: &Config,
) -> std::result::Result<HashMap<String, PathBuf>, Box<dyn std::error::Error>> {
    let mut map = HashMap::new();
    let base_path = Path::new(&config.base_path);
    let current_term = &config.current_term;
    for course in &config.courses {
        map.insert(
            course.name.clone(),
            base_path.join(current_term).join(&course.name),
        );
    }
    Ok(map)
}

fn parse_event(
    event: DebouncedEvent,
) -> std::result::Result<ParseEventResult, Box<dyn std::error::Error>> {
    if let EventKind::Create(CreateKind::File) | EventKind::Modify(ModifyKind::Any) =
        event.event.kind
    {
        log::info!("Running some code");
        let path = event
            .event
            .paths
            .get(0)
            .and_then(|p| p.to_str())
            .ok_or("Invalid path")?;

        let data = get_where_froms_attribute(path)?.ok_or("Attribute not found")?;

        let reader = io::Cursor::new(data);
        let plist_value = plist::Value::from_reader(reader)
            .map_err(|e| format!("Failed to parse plist: {}", e))?;

        if let Value::Array(array) = plist_value {
            for item in array {
                if let Value::String(url) = item {
                    let file_name = extract_filename(path).ok_or("Failed to extract filename")?;
                    return Ok(ParseEventResult::File(File {
                        name: file_name.to_string(),
                        url,
                        path: path.into(),
                    }));
                }
            }
        }
    }

    Ok(ParseEventResult::Empty)
}

fn craft_staring_prompt(courses: Vec<Course>) -> String {
    let prompt = format!("
    Your job is to determine whether a downloaded folder is a file that corresponds to one of the following courses: {:#?}
    \n
    If so, return the course name. If not, return the string NONE.\n
    You will be given the URL of where the file was downloaded from and the name of the file.
    If the URL contains 'learn.uwaterloo.ca', it is highly likely but not guaranteed that the file is a course file.
    You should check to see if the file name or the URL contains the course code or the course name.", courses);
    return prompt.to_string();
}

fn get_file_contents(file_path: &str) -> std::result::Result<Vec<u8>, std::io::Error> {
    std::fs::read(file_path)
}

fn grep_course_code(
    file: &File,
    courses: Vec<Course>,
) -> std::result::Result<GrepCourseCodeResult, Box<dyn std::error::Error>> {
    let sanitized_name: String = file.name.chars().filter(|c| !c.is_whitespace()).collect();
    for course in courses {
        if sanitized_name.contains(&course.name) || file.url.contains(&course.name) {
            return Ok(GrepCourseCodeResult::Course(course));
        }
    }
    Ok(GrepCourseCodeResult::Empty)
}

fn get_where_froms_attribute(
    file_path: &str,
) -> std::result::Result<std::option::Option<Vec<u8>>, std::io::Error> {
    xattr::get(file_path, "com.apple.metadata:kMDItemWhereFroms")
}

fn extract_filename(file_path: &str) -> Option<&str> {
    file_path.rsplit('/').next()
}
