use clap::Parser;
use cli::Commands;
use notify::{Error, FsEventWatcher};
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
use std::{
    path::Path,
    sync::mpsc::{channel, Sender},
    time::Duration,
};
pub mod cli;
pub mod setup;
use tokio;

#[tokio::main]
async fn main() {
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

fn start_monitor() -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Starting monitor...");
    let config = setup::read_config()?;
    log::info!("Read config");
    let download_path = config.download_path;
    // let base_path = config.base_path;
    log::info!("Starting monitor on {}", download_path.to_str().unwrap());
    let (tx, rx) = channel();

    let _debouncer =
        create_debouncer(download_path, tx.clone()).expect("Failed to create debouncer");

    for result in rx {
        match result {
            Ok(events) => events.iter().for_each(|event| log::info!("{event:?}")),

            Err(errors) => errors.iter().for_each(|error| log::error!("{error:?}")),
        }
    }

    Ok(())
}

fn create_debouncer<P: AsRef<Path>>(
    path: P,
    tx: Sender<Result<Vec<DebouncedEvent>, Vec<Error>>>,
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
