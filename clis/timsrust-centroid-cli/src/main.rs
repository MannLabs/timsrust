fn main() {
    if let Err(error) = timsrust_centroid_cli::CLI::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
