use std::path::PathBuf;
use std::time::Instant;

use reforger_language_server::pack::{PakArchive, PakSelection};

fn main() {
    let paths = std::env::args_os().skip(1).map(PathBuf::from);
    let mut supplied = false;

    for path in paths {
        supplied = true;
        let started = Instant::now();
        match PakArchive::inspect(&path) {
            Ok(archive) => match archive.select(PakSelection::scripts()) {
                Ok(scripts) => println!(
                    "{}: {} catalogue entries; {} .c entries; {:.3}s",
                    path.display(),
                    archive.entries().len(),
                    scripts.len(),
                    started.elapsed().as_secs_f64()
                ),
                Err(error) => eprintln!("{}: selection failed: {error}", path.display()),
            },
            Err(error) => eprintln!("{}: inspection failed: {error}", path.display()),
        }
    }

    if !supplied {
        eprintln!("usage: cargo run --example pack_catalogue_report -- <archive.pak> [...]");
        std::process::exit(2);
    }
}
