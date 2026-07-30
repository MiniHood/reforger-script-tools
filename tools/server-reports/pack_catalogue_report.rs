use std::fs::{create_dir_all, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use reforger_language_server::pack::{PakArchive, PakEntry, PakSelection};

const SLOWEST_ENTRY_LIMIT: usize = 10;

struct Options {
    extraction_root: Option<PathBuf>,
    profile_scripts: bool,
    archives: Vec<PathBuf>,
}

struct EntryTiming {
    path: String,
    elapsed: Duration,
    compressed_bytes: u64,
    original_bytes: u64,
    compression: u32,
}

struct ScriptProfile {
    elapsed: Duration,
    compressed_bytes: u64,
    original_bytes: u64,
    compression_counts: std::collections::BTreeMap<u32, usize>,
    slowest: Vec<EntryTiming>,
}

fn main() {
    run(parse_options());
}

fn parse_options() -> Options {
    let mut arguments = std::env::args_os().skip(1);
    let mut options = Options {
        extraction_root: None,
        profile_scripts: false,
        archives: Vec::new(),
    };

    while let Some(argument) = arguments.next() {
        if argument == "--extract-scripts" {
            options.extraction_root = Some(match arguments.next() {
                Some(path) => PathBuf::from(path),
                None => usage(),
            });
        } else if argument == "--profile-scripts" {
            options.profile_scripts = true;
        } else {
            options.archives.push(PathBuf::from(argument));
        }
    }

    if options.archives.is_empty() {
        usage();
    }
    options
}

fn run(options: Options) {
    for path in options.archives {
        let inspect_started = Instant::now();
        let archive = match PakArchive::inspect(&path) {
            Ok(archive) => archive,
            Err(error) => {
                eprintln!("{}: inspection failed: {error}", path.display());
                continue;
            }
        };
        let inspect_elapsed = inspect_started.elapsed();

        let selection_started = Instant::now();
        let scripts = match archive.select(PakSelection::scripts()) {
            Ok(scripts) => scripts,
            Err(error) => {
                eprintln!("{}: selection failed: {error}", path.display());
                continue;
            }
        };
        let selection_elapsed = selection_started.elapsed();

        println!(
            "{}: catalogue={} scripts={} inspect={} select={}",
            path.display(),
            archive.entries().len(),
            scripts.len(),
            format_duration(inspect_elapsed),
            format_duration(selection_elapsed),
        );

        if let Some(root) = &options.extraction_root {
            match extract_scripts(&archive, &scripts, root) {
                Ok(count) => println!("  extraction: files={count} output={}", root.display()),
                Err(error) => eprintln!("  extraction failed: {error}"),
            }
        }

        if options.profile_scripts {
            match profile_scripts(&archive, &scripts) {
                Ok(profile) => print_profile(&profile),
                Err(error) => eprintln!("  profiling failed: {error}"),
            }
        }
    }
}

fn extract_scripts(
    archive: &PakArchive,
    scripts: &[PakEntry],
    root: &Path,
) -> Result<usize, String> {
    let mut count = 0;
    for entry in scripts {
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
        count += 1;
    }
    Ok(count)
}

fn profile_scripts(archive: &PakArchive, scripts: &[PakEntry]) -> Result<ScriptProfile, String> {
    let started = Instant::now();
    let mut compressed_bytes = 0;
    let mut original_bytes = 0;
    let mut compression_counts = std::collections::BTreeMap::new();
    let mut slowest = Vec::with_capacity(scripts.len());

    for entry in scripts {
        let entry_started = Instant::now();
        archive
            .read_to(entry, &mut io::sink())
            .map_err(|error| format!("{}: {error}", entry.logical_path()))?;
        compressed_bytes += entry.compressed_length();
        original_bytes += entry.original_length();
        *compression_counts.entry(entry.compression()).or_default() += 1;
        slowest.push(EntryTiming {
            path: entry.logical_path().to_owned(),
            elapsed: entry_started.elapsed(),
            compressed_bytes: entry.compressed_length(),
            original_bytes: entry.original_length(),
            compression: entry.compression(),
        });
    }

    slowest.sort_unstable_by(|left, right| right.elapsed.cmp(&left.elapsed));
    slowest.truncate(SLOWEST_ENTRY_LIMIT);
    Ok(ScriptProfile {
        elapsed: started.elapsed(),
        compressed_bytes,
        original_bytes,
        compression_counts,
        slowest,
    })
}

fn print_profile(profile: &ScriptProfile) {
    let elapsed_seconds = profile.elapsed.as_secs_f64();
    let mib_per_second = if elapsed_seconds == 0.0 {
        0.0
    } else {
        profile.original_bytes as f64 / (1024.0 * 1024.0) / elapsed_seconds
    };
    println!(
        "  profile: read+decode={} compressed={} original={} throughput={mib_per_second:.1} MiB/s",
        format_duration(profile.elapsed),
        format_bytes(profile.compressed_bytes),
        format_bytes(profile.original_bytes),
    );
    println!(
        "  compression: {}",
        format_compression_counts(&profile.compression_counts)
    );
    println!("  slowest script reads:");
    for entry in &profile.slowest {
        println!(
            "    {} | {} | {} -> {} | compression={}",
            format_duration(entry.elapsed),
            entry.path,
            format_bytes(entry.compressed_bytes),
            format_bytes(entry.original_bytes),
            entry.compression,
        );
    }
}

fn format_compression_counts(counts: &std::collections::BTreeMap<u32, usize>) -> String {
    counts
        .iter()
        .map(|(compression, count)| format!("{compression}={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_duration(duration: Duration) -> String {
    format!("{:.3} ms", duration.as_secs_f64() * 1_000.0)
}

fn format_bytes(bytes: u64) -> String {
    format!("{:.2} MiB", bytes as f64 / (1024.0 * 1024.0))
}

fn usage() -> ! {
    eprintln!(
        "usage: cargo run --example pack_catalogue_report -- [--profile-scripts] [--extract-scripts <output-root>] <archive.pak> [...]"
    );
    std::process::exit(2);
}
