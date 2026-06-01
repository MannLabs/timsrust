mod args;
mod logging;
mod panic;
pub mod prelude;
mod threads;

use clap::{ArgMatches, Command};

pub use args::CommonArgs;
pub use logging::init_logging;
pub use panic::install_panic_hook;
pub use threads::configure_thread_pool;

/// Context derived from common CLI flags
#[derive(Debug, Clone)]
pub struct CliContext {
    pub verbosity: u8,
    pub quiet: bool,
}

/// Create the base clap Command shared by all binaries
pub fn base_command(bin_name: &'static str) -> Command {
    args::base_command(bin_name)
}

/// Initialize logging and return common CLI context
pub fn init_from_matches(matches: &ArgMatches) -> CliContext {
    let args = CommonArgs::from_matches(matches);
    install_panic_hook();
    init_logging(args.verbosity, args.quiet);

    // Configure rayon thread pool
    if let Err(e) = configure_thread_pool(args.threads) {
        log::warn!("Failed to configure thread pool: {}", e);
    }

    CliContext {
        verbosity: args.verbosity,
        quiet: args.quiet,
    }
}

/// Parse arguments and automatically initialize logging, panic handler, and thread pool.
/// Returns the parsed matches with initialization already complete.
pub fn parse_and_init(command: Command) -> ArgMatches {
    let matches = command.get_matches();
    init_from_matches(&matches);
    matches
}
