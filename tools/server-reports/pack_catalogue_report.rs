use std::fs::{create_dir_all, OpenOptions};
use std::path::PathBuf;
use std::time::Instant;

use reforger_language_server::pack::{PakArchive, PakSelection};

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let extraction_root = match arguments.next() {
        Some(argument) if argument == "--extract-scripts" => match arguments.next() {
            Some(path) => Some(PathBuf::from(path)),
            None => usage(),
        },
        Some(argument) => {
            let mut paths = vec![PathBuf::from(argument)];
            paths.extend(arguments.map(PathBuf::from));
            return run(paths, None);
        }
        None => usage(),
    };
    run(arguments.map(PathBuf::from), extraction_root);
}

fn run(paths: impl IntoIterator<Item = PathBuf>, extraction_root: Option<PathBuf>) {
    let mut supplied = false;

    for path in paths {
        supplied = true;
        let started = Instant::now();
        match PakArchive::inspect(&path) {
            Ok(archive) => match archive.select(PakSelection::scripts()) {
                Ok(scripts) => {
                    let extracted = (|| -> Result<usize, String> {
                        let Some(root) = extraction_root.as_ref() else {
                            return Ok(0);
                        };
                        let mut count = 0;
                        for entry in &scripts {
                            count += extract(&archive, entry, root)?;
                        }
                        Ok(count)
                    })();
                    match extracted {
                        Ok(count) => println!(
                            "{}: {} catalogue entries; {} .c entries; {} extracted; {:.3}s",
                            path.display(),
                            archive.entries().len(),
                            scripts.len(),
                            count,
                            started.elapsed().as_secs_f64()
                        ),
                        Err(error) => eprintln!("{}: extraction failed: {error}", path.display()),
                    }
                }
                Err(error) => eprintln!("{}: selection failed: {error}", path.display()),
            },
            Err(error) => eprintln!("{}: inspection failed: {error}", path.display()),
        }
    }

    if !supplied {
        usage();
    }
}

fn extract(
    archive: &PakArchive,
    entry: &reforger_language_server::pack::PakEntry,
    root: &std::path::Path,
) -> Result<usize, String> {
    let destination = root.join(entry.logical_path());
    let parent = destination
        .parent()
        .ok_or_else(|| format!("script has no destination parent: {}", entry.logical_path()))?;
    create_dir_all(parent).map_err(|error| error.to_string())?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .map_err(|error| format!("{}: {error}", destination.display()))?;
    archive
        .read_to(entry, &mut output)
        .map_err(|error| format!("{}: {error}", entry.logical_path()))?;
    Ok(1)
}

fn usage() -> ! {
    eprintln!(
        "usage: cargo run --example pack_catalogue_report -- [--extract-scripts <output-dir>] <archive.pak> [...]"
    );
    std::process::exit(2);
}
