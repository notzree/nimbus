use clap::Parser;
use cli::Commands;
use monitor::start_monitor;
pub mod cli;
pub mod monitor;
pub mod review;
pub mod setup;
use dotenv::dotenv;

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
            review::read_commands().unwrap();
        }
        Commands::Start => match start_monitor() {
            Ok(_) => log::info!("monitor started"),
            Err(e) => log::error!("Failed to start monitor: {}", e),
        },
    }
}
