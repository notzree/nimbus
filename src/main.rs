use clap::Parser;
use cli::Commands;
use core::panic;
use dirs::download_dir;
use notify::{Error, FsEventWatcher};
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
use std::thread;
use std::{
    path::Path,
    sync::mpsc::{channel, sync_channel, Sender},
    time::Duration,
};
pub mod cli;
pub mod setup;
extern crate daemonize;
use daemonize::Daemonize;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let nimbus = cli::Nimbus::parse();

    match nimbus.command {
        Commands::Config => {
            // Handle 'nimbus config' here
            let (tx, rx) = sync_channel(1);
            setup::setup_nimbus(tx).unwrap();
            match rx.recv() {
                Ok(start_daemon) => {
                    if start_daemon {
                        log::info!("Starting daemon...");
                        //TODO: Inclue function to start daemon process
                    }
                }
                Err(e) => panic!("Failed to receive: {}", e),
            }
        }
        Commands::Review => {
            // Handle 'nimbus review' here
            log::info!("Reviewing...");
        }
        Commands::Daemon => {
            // Handle 'nimbus daemon' here
            log::info!("Starting daemon...");
        }
    }

    // let config = cli::parse_yaml(args.yaml_path).unwrap();
    // let download_path = setup::setup_directory(config).unwrap();

    // let (tx, rx) = channel();

    // let _debouncer =
    //     create_debouncer(download_path, tx.clone()).expect("Failed to create debouncer");

    // for result in rx {
    //     match result {
    //         Ok(events) => events.iter().for_each(|event| log::info!("{event:?}")),

    //         Err(errors) => errors.iter().for_each(|error| log::error!("{error:?}")),
    //     }
    // }
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

fn start_daemon() {}

fn watch<P: AsRef<Path>>(path: P) -> notify::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    // Create a new debounced file watcher with a timeout of 2 seconds.
    // The tickrate will be selected automatically, as well as the underlying watch implementation.
    let mut debouncer = new_debouncer(Duration::from_secs(2), None, tx)?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    debouncer
        .watcher()
        .watch(path.as_ref(), RecursiveMode::Recursive)?;

    // Initialize the file id cache for the same path. This will allow the debouncer to stitch together move events,
    // even if the underlying watch implementation doesn't support it.
    // Without the cache and with some watch implementations,
    // you may receive `move from` and `move to` events instead of one `move both` event.
    debouncer
        .cache()
        .add_root(path.as_ref(), RecursiveMode::Recursive);

    // print all events and errors
    for result in rx {
        match result {
            Ok(events) => events.iter().for_each(|event| log::info!("{event:?}")),
            Err(errors) => errors.iter().for_each(|error| log::error!("{error:?}")),
        }
    }

    Ok(())
}
