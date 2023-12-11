use chatgpt::prelude::*;
use clap::Parser;
use cli::Commands;
use notify::event::{CreateKind, EventKind};
use notify::{Error, FsEventWatcher};
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
use plist::Value;
use setup::read_config;
use std::fs::metadata;
use std::io;
use std::{
    path::Path,
    sync::mpsc::{channel, Sender},
    time::Duration,
};

pub mod cli;
pub mod setup;
use dotenv::dotenv;
use tokio::*;

#[derive(Debug, Clone)]
pub struct File {
    name: String,
    url: String,
    path: String,
}

enum ParseEventResult {
    File(File),
    Empty,
}

// TODO: Make an Enum out of the courses that have been added to the config file

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let nimbus = cli::Nimbus::parse();

    match nimbus.command {
        Commands::Config => {
            // Handle 'nimbus config' here
            setup::setup_nimbus().await.unwrap();
        }
        Commands::Review => {
            // Handle 'nimbus review' here
            log::info!("Reviewing...");
        }
        Commands::Start => match start_monitor() {
            Ok(_) => log::info!("monitor started"),
            Err(e) => log::error!("Failed to start monitor: {}", e),
        },
    }
}

fn start_monitor() -> std::result::Result<(), Box<dyn std::error::Error>> {
    log::info!("Starting monitor...");
    let gpt_client =
        ChatGPT::new(std::env::var("GPT_API_KEY").expect("OpenAI API key required")).unwrap();
    let mut conversation: Conversation = gpt_client.new_conversation_directed(
        "You are a LLM designed to categorize downloaded files into their  ",
    );
    let (action_tx, mut action_rx) = tokio::sync::mpsc::unbounded_channel();
    let config = setup::read_config()?;
    let download_path = config.download_path;
    log::info!("Starting monitor on {}", download_path.to_str().unwrap());
    let (debouncer_tx, debouncer_rx) = channel();

    let _debouncer =
        create_debouncer(download_path, debouncer_tx.clone()).expect("Failed to create debouncer");

    tokio::spawn(async move {
        while let Some(event) = action_rx.recv().await {
            let file = parse_event(event);
            match file {
                Ok(ParseEventResult::File(file)) => {
                    
                }
                Ok(_) => {}
                Err(e) => {
                    log::error!("Error: {}", e);
                }
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

fn parse_event(
    event: DebouncedEvent,
) -> std::result::Result<ParseEventResult, Box<dyn std::error::Error>> {
    if let EventKind::Create(CreateKind::File) = event.event.kind {
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
                        path: path.to_string(),
                    }));
                }
            }
        }
    }

    Ok(ParseEventResult::Empty)
}

fn craft_staring_prompt() -> String {
    let config = read_config().unwrap();
    let courses = config.courses;

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

fn grep_course_code(file: File)->

fn get_where_froms_attribute(
    file_path: &str,
) -> std::result::Result<std::option::Option<Vec<u8>>, std::io::Error> {
    xattr::get(file_path, "com.apple.metadata:kMDItemWhereFroms")
}

fn extract_filename(file_path: &str) -> Option<&str> {
    file_path.rsplit('/').next()
}
