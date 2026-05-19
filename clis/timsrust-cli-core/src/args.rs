use clap::{Arg, ArgAction, ArgMatches, Command};

pub struct CommonArgs {
    pub verbosity: u8,
    pub quiet: bool,
    pub threads: i32,
}

impl CommonArgs {
    pub fn from_matches(m: &ArgMatches) -> Self {
        let verbosity = m.get_count("verbose") + 1;
        let quiet = m.get_flag("quiet");
        let threads = m
            .get_one::<String>("threads")
            .and_then(|s| s.parse::<i32>().ok())
            .expect("Invalid thread count");
        Self {
            verbosity,
            quiet,
            threads,
        }
    }
}

pub(crate) fn base_command(bin_name: &'static str) -> Command {
    Command::new(bin_name)
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Increase verbosity (-v, -vv)")
                .action(ArgAction::Count)
                .display_order(900),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Suppress non-error output")
                .action(ArgAction::SetTrue)
                .display_order(901),
        )
        // .arg(
        //     Arg::new("color")
        //         .long("color")
        //         .value_parser(["auto", "always", "never"])
        //         .default_value("auto")
        //         .help("Control colored output"),
        // )
        .arg(
            Arg::new("threads")
                .short('t')
                .long("threads")
                .default_value("0")
                .help("Use 0 for all or negative to define how many to leave available")
                .display_order(902),
        )
}
