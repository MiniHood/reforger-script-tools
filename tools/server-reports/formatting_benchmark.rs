use reforger_language_server::formatting::format_comment_region;
use reforger_language_server::lexer::TextSpan;
use std::env;
use std::hint::black_box;
use std::time::Instant;

const DEFAULT_ITERATIONS: usize = 31;
const DEFAULT_LINES: usize = 2_000;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    let source = synthetic_comment_region(args.lines);
    let range = TextSpan::new(0, source.len());
    let expected = edit_fingerprint(&format_comment_region(&source, range));
    let mut samples = Vec::with_capacity(args.iterations);

    for _ in 0..args.iterations {
        let started = Instant::now();
        let edits = format_comment_region(black_box(&source), range);
        let elapsed = started.elapsed().as_micros();
        let fingerprint = edit_fingerprint(&edits);
        if fingerprint != expected {
            return Err(format!(
                "Formatting projection changed between iterations: expected {expected:016x}, got {fingerprint:016x}"
            ));
        }
        samples.push(elapsed);
        black_box(edits);
    }

    println!("Formatting benchmark");
    println!("bytes={}", source.len());
    println!("lines={}", args.lines);
    println!("iterations={}", args.iterations);
    println!("edit_fingerprint={expected:016x}");
    print_metric("wall_us", samples);
    Ok(())
}

fn synthetic_comment_region(lines: usize) -> String {
    let mut source = String::with_capacity(lines.saturating_mul(48));
    for index in 0..lines {
        if index % 8 == 0 {
            source.push_str("\t//! Group heading\n");
        } else {
            source.push_str("  //! Documentation payload that stays unchanged\n");
        }
    }
    source
}

fn edit_fingerprint(edits: &[reforger_language_server::formatting::FormattingEdit]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in edits.iter().flat_map(|edit| {
        edit.span
            .start
            .to_le_bytes()
            .into_iter()
            .chain(edit.span.end.to_le_bytes())
            .chain(edit.replacement.as_bytes().iter().copied())
    }) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn print_metric(name: &str, values: Vec<u128>) {
    let minimum = percentile(values.clone(), 0);
    let median = percentile(values.clone(), 50);
    let p95 = percentile(values.clone(), 95);
    let maximum = percentile(values, 100);
    println!("{name}=min:{minimum},median:{median},p95:{p95},max:{maximum}");
}

fn percentile(mut values: Vec<u128>, percentile: usize) -> u128 {
    values.sort_unstable();
    let index = if values.len() == 1 {
        0
    } else {
        ((values.len() - 1) * percentile).div_ceil(100)
    };
    values[index]
}

struct Args {
    iterations: usize,
    lines: usize,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut iterations = DEFAULT_ITERATIONS;
        let mut lines = DEFAULT_LINES;
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            let value = match arg.as_str() {
                "--iterations" | "--lines" => args
                    .next()
                    .ok_or_else(|| format!("{arg} requires a number"))?,
                "--help" | "-h" => {
                    println!(
                        "Usage: cargo run --manifest-path server/Cargo.toml --example formatting_benchmark -- [--lines <n>] [--iterations <n>]"
                    );
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            };
            let parsed = value
                .parse::<usize>()
                .map_err(|_| format!("Invalid {arg} value: {value}"))?;
            if parsed == 0 {
                return Err(format!("{arg} must be greater than zero"));
            }
            if arg == "--iterations" {
                iterations = parsed;
            } else {
                lines = parsed;
            }
        }
        Ok(Self { iterations, lines })
    }
}
