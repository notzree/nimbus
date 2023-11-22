use clap::{Parser, Subcommand};
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
extern crate clap;
#[derive(Debug, serde::Deserialize, Subcommand)]
pub enum Commands {
    //Configures nimbus
    Config,
    //Starts a reveiw
    Review,
    //Starts the daemon
    Start,
}

#[derive(Parser)]
pub struct Nimbus {
    #[clap(subcommand)]
    pub command: Commands,
}
