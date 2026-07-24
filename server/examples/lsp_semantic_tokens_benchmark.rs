use reforger_language_server::index_build::{build_index, IndexBuildConfig, IndexSourceRoot};
use reforger_language_server::lsp::{
    semantic_tokens_for_source_with_external, LspSemanticTokenTimings,
};
use reforger_language_server::model::{SourceKind, SOURCE_PRIORITY_GAME_DATA};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

const DEFAULT_ITERATIONS: usize = 7;
const DEFAULT_STORAGE_RELATIVE_PATH: &str =
    "Code/User/globalStorage/undefined_publisher.reforger-sript-tools/game-data/scripts";

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    if !args.file.is_file() {
        return Err(format!(
            "Benchmark file does not exist: {}",
            args.file.display()
        ));
    }
    if !args.scripts.is_dir() {
        return Err(format!(
            "Game-data scripts folder does not exist: {}",
            args.scripts.display()
        ));
    }

    let source = fs::read_to_string(&args.file)
        .map_err(|error| format!("Failed to read {}: {error}", args.file.display()))?;
    let index_start = Instant::now();
    let external_index = build_index(&IndexBuildConfig {
        roots: vec![IndexSourceRoot::new(
            &args.scripts,
            SourceKind::GameData,
            SOURCE_PRIORITY_GAME_DATA,
        )],
    })
    .map(|result| result.index)?;
    let index_ms = index_start.elapsed().as_millis();

    let _warmup = semantic_tokens_for_source_with_external(&source, Some(&external_index));
    let mut samples = Vec::with_capacity(args.iterations);
    let mut expected = None;
    for _ in 0..args.iterations {
        let started = Instant::now();
        let projection = semantic_tokens_for_source_with_external(&source, Some(&external_index));
        let wall_ms = started.elapsed().as_millis();
        let fingerprint = projection_fingerprint(
            projection.token_count,
            projection.timings.identifier_resolver_calls,
            &projection.tokens.data,
        );
        let stable = (
            projection.token_count,
            projection.timings.identifier_resolver_calls,
            fingerprint,
        );
        if let Some(expected) = expected {
            if stable != expected {
                return Err(format!(
                    "Semantic projection changed between iterations: expected {expected:?}, got {stable:?}"
                ));
            }
        } else {
            expected = Some(stable);
        }
        samples.push(Sample {
            wall_ms,
            timings: projection.timings,
        });
    }

    let (token_count, resolver_calls, fingerprint) =
        expected.ok_or_else(|| "Benchmark produced no samples".to_string())?;
    println!("Semantic-token benchmark");
    println!("file={}", args.file.display());
    println!("bytes={}", source.len());
    println!("iterations={}", args.iterations);
    println!("external_index_build_ms={index_ms}");
    println!("tokens={token_count}");
    println!("resolver_calls={resolver_calls}");
    println!("token_fingerprint={fingerprint:016x}");
    print_metric(
        "wall_ms",
        samples.iter().map(|sample| sample.wall_ms).collect(),
    );
    print_metric(
        "resolver_ms",
        samples
            .iter()
            .map(|sample| sample.timings.resolver_ms)
            .collect(),
    );
    print_metric(
        "resolver_context_ms",
        samples
            .iter()
            .map(|sample| sample.timings.resolver_context_ms)
            .collect(),
    );
    print_metric(
        "resolver_scope_ms",
        samples
            .iter()
            .map(|sample| sample.timings.resolver_scope_ms)
            .collect(),
    );
    print_metric(
        "resolver_member_ms",
        samples
            .iter()
            .map(|sample| sample.timings.resolver_member_ms)
            .collect(),
    );
    print_metric(
        "resolver_external_ms",
        samples
            .iter()
            .map(|sample| sample.timings.resolver_external_ms)
            .collect(),
    );

    let median_resolver_ms = percentile(
        samples
            .iter()
            .map(|sample| sample.timings.resolver_ms)
            .collect(),
        50,
    );
    if let Some(maximum) = args.max_median_resolver_ms {
        if median_resolver_ms > maximum {
            return Err(format!(
                "FAIL median resolver latency {median_resolver_ms} ms exceeds budget {maximum} ms"
            ));
        }
        println!(
            "PASS median resolver latency {median_resolver_ms} ms is within budget {maximum} ms"
        );
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Sample {
    wall_ms: u128,
    timings: LspSemanticTokenTimings,
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

fn projection_fingerprint(token_count: usize, resolver_calls: usize, data: &[u32]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in token_count
        .to_le_bytes()
        .into_iter()
        .chain(resolver_calls.to_le_bytes())
        .chain(data.iter().flat_map(|value| value.to_le_bytes()))
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

struct Args {
    scripts: PathBuf,
    file: PathBuf,
    iterations: usize,
    max_median_resolver_ms: Option<u128>,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut scripts = None;
        let mut file = None;
        let mut iterations = DEFAULT_ITERATIONS;
        let mut max_median_resolver_ms = None;
        let mut args = env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--scripts" => {
                    scripts = Some(PathBuf::from(
                        args.next()
                            .ok_or_else(|| "--scripts requires a path".to_string())?,
                    ));
                }
                "--file" => {
                    file = Some(PathBuf::from(
                        args.next()
                            .ok_or_else(|| "--file requires a path".to_string())?,
                    ));
                }
                "--iterations" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--iterations requires a number".to_string())?;
                    iterations = value
                        .parse::<usize>()
                        .map_err(|_| format!("Invalid --iterations value: {value}"))?;
                    if iterations == 0 {
                        return Err("--iterations must be greater than zero".to_string());
                    }
                }
                "--max-median-resolver-ms" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--max-median-resolver-ms requires a number".to_string())?;
                    max_median_resolver_ms =
                        Some(value.parse::<u128>().map_err(|_| {
                            format!("Invalid --max-median-resolver-ms value: {value}")
                        })?);
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }

        Ok(Self {
            scripts: scripts.unwrap_or_else(default_scripts_path),
            file: file.ok_or_else(|| "--file is required".to_string())?,
            iterations,
            max_median_resolver_ms,
        })
    }
}

fn default_scripts_path() -> PathBuf {
    if let Some(app_data) = env::var_os("APPDATA") {
        PathBuf::from(app_data).join(DEFAULT_STORAGE_RELATIVE_PATH)
    } else {
        PathBuf::from(DEFAULT_STORAGE_RELATIVE_PATH)
    }
}

fn print_help() {
    println!(
        "Usage: cargo run --manifest-path server/Cargo.toml --example lsp_semantic_tokens_benchmark -- --file <path> [--scripts <path>] [--iterations <n>] [--max-median-resolver-ms <n>]"
    );
}
