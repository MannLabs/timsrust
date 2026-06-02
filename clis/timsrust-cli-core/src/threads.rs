/// Configure rayon thread pool based on threads argument:
/// - 0: use all available threads
/// - positive: use exactly that many threads
/// - negative: leave that many threads available (total - abs(threads))
pub fn configure_thread_pool(
    threads: i32,
) -> Result<(), rayon::ThreadPoolBuildError> {
    let num_threads = if threads == 0 {
        // Use all available threads
        0
    } else if threads > 0 {
        // Use exactly this many threads
        threads as usize
    } else {
        // Leave abs(threads) available
        let available = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        (available as i32 - threads).max(1) as usize
    };

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
}
