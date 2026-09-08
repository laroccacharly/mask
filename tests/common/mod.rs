use std::time::Instant;

pub fn timed<T>(label: &str, run: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    let value = run();
    eprintln!("{label} took {:.2?}", started.elapsed());
    value
}
