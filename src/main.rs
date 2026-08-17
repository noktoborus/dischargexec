extern crate starship_battery as battery;

use clap::{Parser, Subcommand};
use cross_exec::CommandExt;
use log::LevelFilter;
use log::{debug, info};
use pidlock::Pidlock;
use simplelog::Config;
use simplelog::SimpleLogger;
use std::io;
use std::process::Command;
use std::thread;
use std::time::Duration;

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Watch battery state and run command if battery is discharging
    #[command(arg_required_else_help = true)]
    Exec { command: String },
    /// Abort running watcher
    Abort,
}

fn monitor_battery() -> battery::Result<()> {
    let manager = battery::Manager::new()?;
    let mut battery = match manager.batteries()?.next() {
        Some(Ok(battery)) => battery,
        Some(Err(e)) => {
            eprintln!("Unable to access battery information");
            return Err(e);
        }
        None => {
            eprintln!("Unable to find any batteries");
            return Err(io::Error::from(io::ErrorKind::NotFound).into());
        }
    };

    // Threshold is needed to avoid power jitter:
    // May Charging/Discharging states jitter appear on 100% charge level
    let discharging_threshold = 60;
    let mut discharging_states = 0;

    loop {
        manager.refresh(&mut battery)?;

        if battery.state() == battery::State::Discharging {
            discharging_states += 1;
        } else {
            discharging_states = 0;
        }

        debug!("states: {}/{}", discharging_states, discharging_threshold);
        if discharging_states == discharging_threshold {
            return Ok(());
        }

        thread::sleep(Duration::from_secs(1));
    }
}

fn main() -> battery::Result<()> {
    SimpleLogger::init(LevelFilter::Info, Config::default()).unwrap();
    let cli = Cli::parse();

    let temp_dir = std::env::temp_dir();
    let lock_path = temp_dir.join("dischargexec.pid");
    let mut lock = Pidlock::new_validated(&lock_path).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Lock file is not valid: {}", e),
        )
    })?;

    match &cli.command {
        Commands::Exec { command } => match lock.acquire() {
            Ok(_) => {
                let Some(command) = shlex::split(command) else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Bad Command",
                    ))?
                };
                let (prog, args) = command.split_first().ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "Empty command")
                })?;

                info!("Wait discharge event");
                monitor_battery()?;
                info!("Discharding detected, execute command: {:?}", command);

                let err = Command::new(prog).args(args).cross_exec();
                Err(err.into())
            }

            Err(pidlock::PidlockError::LockExists) => Err(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                format!("Another instance is already running"),
            ))?,

            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to acquire lock: {}", e),
            ))?,
        },
        Commands::Abort => match lock.get_owner() {
            Ok(Some(pid)) => {
                info!("Kill {}", pid);
                cross_spawn::kill(pid.try_into().unwrap()).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to kill: {}", e))
                })?;
                Ok(())
            }
            Ok(None) => Err(std::io::Error::new(
                std::io::ErrorKind::AddrNotAvailable,
                format!("No running instance found"),
            ))?,
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get lock owner: {}", e),
            ))?,
        },
    }
}
