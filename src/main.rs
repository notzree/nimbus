use chatgpt::prelude::*;
use clap::Parser;
use cli::Commands;
use notify::event::{CreateKind, EventKind};
use notify::{Error, FsEventWatcher};
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
use plist::Value;
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
    let (action_tx, mut action_rx) = tokio::sync::mpsc::unbounded_channel();
    let config = setup::read_config()?;
    let download_path = config.download_path;
    log::info!("Starting monitor on {}", download_path.to_str().unwrap());
    let (debouncer_tx, debouncer_rx) = channel();

    let _debouncer =
        create_debouncer(download_path, debouncer_tx.clone()).expect("Failed to create debouncer");

    tokio::spawn(async move {
        while let Some(event) = action_rx.recv().await {
            // Do something with the event
            handle_event(event, gpt_client.clone());
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

fn handle_event(
    event: DebouncedEvent,
    client: ChatGPT,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    match event.event.kind {
        EventKind::Create(kind) => {
            if kind == CreateKind::File {
                // log::info!("Downloaded file");
                let path = event.event.paths[0].to_str().unwrap();
                let metadata = std::fs::metadata(path)?;
                match get_where_froms_attribute(path)? {
                    Some(data) => {
                        log::info!("Attribute found");
                        let reader = io::Cursor::new(data);
                        match plist::Value::from_reader(reader) {
                            Ok(value) => {
                                if let Value::Array(array) = value {
                                    for item in array {
                                        if let Value::String(url) = item {
                                            log::info!("URL: {}", url);
                                        }
                                    }
                                }
                            }
                            Err(e) => eprintln!("Failed to parse plist: {}", e),
                        }
                    }
                    None => println!("Attribute not found"),
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn get_where_froms_attribute(file_path: &str) -> io::Result<Option<Vec<u8>>> {
    xattr::get(file_path, "com.apple.metadata:kMDItemWhereFroms")
}
