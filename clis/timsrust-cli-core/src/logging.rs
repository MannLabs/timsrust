use env_logger::{Builder, Env};
use log::LevelFilter;

pub fn init_logging(verbosity: u8, quiet: bool) {
    // env_logger::Builder::from_default_env()
    // .filter_level(log::LevelFilter::Info)
    // .init();
    let env = Env::default().filter_or("RUST_LOG", "warn");
    let mut builder = Builder::from_env(env);
    let level = if quiet {
        LevelFilter::Error
    } else {
        match verbosity {
            0 => LevelFilter::Warn,
            1 => LevelFilter::Info,
            2 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        }
    };
    builder.filter_level(level);
    // builder.format(|buf, record| {
    //     writeln!(buf, "{:>5} {}", record.level(), record.args())
    // });
    builder.init();
}
