use reforger_language_server::index_build::{build_index, IndexBuildConfig, IndexSourceRoot};
use reforger_language_server::lsp::{
    completion_report_for_cached_analysis_with_external, file_index_for_source, LspPosition,
};
use reforger_language_server::model::{SourceKind, SOURCE_PRIORITY_GAME_DATA};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    let source = fs::read_to_string(&args.file)
        .map_err(|error| format!("Failed to read {}: {error}", args.file.display()))?;
    let analysis = file_index_for_source(&source);
    let external_started = Instant::now();
    let external_index = build_index(&IndexBuildConfig {
        roots: vec![IndexSourceRoot::new(
            &args.scripts,
            SourceKind::GameData,
            SOURCE_PRIORITY_GAME_DATA,
        )],
    })
    .map(|result| result.index)?;
    let external_elapsed = external_started.elapsed();
    let position = LspPosition {
        line: args.line - 1,
        character: args.character,
    };

    let mut samples = Vec::with_capacity(args.iterations);
    let mut fingerprint = None;
    let mut context = String::new();
    let mut candidates = 0usize;
    for iteration in 0..(args.warmups + args.iterations) {
        let started = Instant::now();
        let report = completion_report_for_cached_analysis_with_external(
            &source,
            &analysis,
            position,
            Some(&external_index),
        );
        let wall = started.elapsed();
        let encoded = serde_json::to_vec(&report.list)
            .map_err(|error| format!("Failed to encode completion list: {error}"))?;
        let current_fingerprint = format!("{:x}", Sha256::digest(encoded));
        if let Some(expected) = &fingerprint {
            if expected != &current_fingerprint {
                return Err(format!(
                    "Completion output changed between iterations: {expected} != {current_fingerprint}"
                ));
            }
        } else {
            fingerprint = Some(current_fingerprint);
            context = report.completion_context.clone();
            candidates = report.candidate_count;
        }
        if iteration >= args.warmups {
            samples.push(Sample {
                wall,
                context: report.timings.context_detection,
                receiver: report.timings.receiver_inference,
                lookup: report.timings.candidate_lookup,
                rendering: report.timings.item_rendering,
                reported_total: report.timings.total,
            });
        }
    }

    println!("file={}", args.file.display());
    println!("line={} character={}", args.line, args.character);
    println!("bytes={}", source.len());
    println!("external_index_ms={}", external_elapsed.as_millis());
    println!("warmups={} iterations={}", args.warmups, args.iterations);
    println!("context={context}");
    println!("candidates={candidates}");
    let fingerprint = fingerprint.unwrap_or_default();
    println!("fingerprint={fingerprint}");
    let wall_median = print_summary("wall_us", durations(&samples, |sample| sample.wall));
    print_summary("context_us", durations(&samples, |sample| sample.context));
    print_summary("receiver_us", durations(&samples, |sample| sample.receiver));
    print_summary("lookup_us", durations(&samples, |sample| sample.lookup));
    print_summary(
        "rendering_us",
        durations(&samples, |sample| sample.rendering),
    );
    print_summary(
        "reported_total_us",
        durations(&samples, |sample| sample.reported_total),
    );
    if let Some(expected) = &args.expect_fingerprint {
        if expected != &fingerprint {
            return Err(format!(
                "Completion fingerprint {fingerprint} did not match expected {expected}"
            ));
        }
    }
    if let Some(max_median_us) = args.max_median_us {
        if wall_median > max_median_us {
            return Err(format!(
                "Completion median {wall_median} us exceeded budget {max_median_us} us"
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Sample {
    wall: Duration,
    context: Duration,
    receiver: Duration,
    lookup: Duration,
    rendering: Duration,
    reported_total: Duration,
}

fn durations(samples: &[Sample], field: impl Fn(&Sample) -> Duration) -> Vec<u128> {
    samples
        .iter()
        .map(|sample| field(sample).as_micros())
        .collect()
}

fn print_summary(label: &str, mut values: Vec<u128>) -> u128 {
    values.sort_unstable();
    let median = values[values.len() / 2];
    let p95 = values[((values.len() - 1) * 95).div_ceil(100)];
    println!(
        "{label} min={} median={median} p95={p95} max={}",
        values[0],
        values[values.len() - 1]
    );
    median
}

struct Args {
    scripts: PathBuf,
    file: PathBuf,
    line: u32,
    character: u32,
    warmups: usize,
    iterations: usize,
    expect_fingerprint: Option<String>,
    max_median_us: Option<u128>,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut scripts = None;
        let mut file = None;
        let mut line = None;
        let mut character = None;
        let mut warmups = 3usize;
        let mut iterations = 21usize;
        let mut expect_fingerprint = None;
        let mut max_median_us = None;
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--scripts" => scripts = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--file" => file = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--line" => line = Some(parse_value(&mut args, &arg)?),
                "--character" => character = Some(parse_value(&mut args, &arg)?),
                "--warmups" => warmups = parse_value(&mut args, &arg)?,
                "--iterations" => iterations = parse_value(&mut args, &arg)?,
                "--expect-fingerprint" => expect_fingerprint = Some(next_value(&mut args, &arg)?),
                "--max-median-us" => max_median_us = Some(parse_value(&mut args, &arg)?),
                "--help" | "-h" => {
                    println!("Usage: cargo run --release --manifest-path server/Cargo.toml --example lsp_completion_benchmark -- --scripts <path> --file <path> --line <one-based-line> --character <zero-based-character> [--warmups <n>] [--iterations <n>] [--expect-fingerprint <sha256>] [--max-median-us <n>]");
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        let line = line.ok_or_else(|| "--line is required".to_string())?;
        if line == 0 {
            return Err("--line must be one-based".to_string());
        }
        if iterations == 0 {
            return Err("--iterations must be greater than zero".to_string());
        }
        Ok(Self {
            scripts: scripts.ok_or_else(|| "--scripts is required".to_string())?,
            file: file.ok_or_else(|| "--file is required".to_string())?,
            line,
            character: character.ok_or_else(|| "--character is required".to_string())?,
            warmups,
            iterations,
            expect_fingerprint,
            max_median_us,
        })
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn parse_value<T: std::str::FromStr>(
    args: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<T, String> {
    let value = next_value(args, option)?;
    value
        .parse()
        .map_err(|_| format!("Invalid {option} value: {value}"))
}
