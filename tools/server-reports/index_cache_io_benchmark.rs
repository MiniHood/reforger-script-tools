use reforger_language_server::index::SourceFileId;
use reforger_language_server::index_build::{
    IndexBuildCounts, IndexBuildResult, IndexBuildShape, IndexBuildSummary, IndexBuildTimings,
};
use reforger_language_server::index_cache::{
    load_game_data_index_cache_with_control, load_or_build_archive_index,
};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_ITERATIONS: usize = 10;

#[derive(Debug)]
struct Args {
    cache_path: PathBuf,
    iterations: usize,
    max_median_load_ms: Option<f64>,
    max_median_write_ms: Option<f64>,
}

#[derive(Debug)]
struct LoadMeasurement {
    total: Duration,
    file_read: Duration,
    decode: Duration,
    validate: Duration,
    map_rebuild: Duration,
    projection: Duration,
    lookup_maps: Duration,
}

#[derive(Debug)]
struct WriteMeasurement {
    total: Duration,
    prepare: Duration,
    compact: Duration,
    payload_prepare: Duration,
    encode_and_write: Duration,
    encode: Duration,
    atomic_write: Duration,
    bytes: u64,
}

fn main() -> Result<(), String> {
    let args = parse_args()?;
    let first = load_once(&args.cache_path)?;
    let template = load_game_data_index_cache_with_control(&args.cache_path, &Default::default())?
        .ok_or_else(|| {
            format!(
                "Cache is not compatible with the current language server: {}",
                args.cache_path.display()
            )
        })?;

    let mut loads = Vec::with_capacity(args.iterations);
    for _ in 0..args.iterations {
        loads.push(load_once(&args.cache_path)?);
    }

    let temp_root = env::temp_dir().join(format!(
        "reforger-index-cache-io-{}-{}",
        std::process::id(),
        unix_nanos()
    ));
    fs::create_dir_all(&temp_root).map_err(|error| {
        format!(
            "Failed to create benchmark folder {}: {error}",
            temp_root.display()
        )
    })?;
    let temp_cache_path = temp_root.join("symbols.bin");
    let source_line_starts = ordered_line_starts(&template.source_line_starts)?;
    let build_summary = IndexBuildSummary {
        totals: IndexBuildCounts {
            files: template.summary.files,
            bytes: template.summary.bytes,
            lossy_files: template.summary.lossy_files,
            parse_diagnostics: template.summary.parse_diagnostics,
            indexed_files: template.index.files().len(),
            indexed_symbols: template.index.symbols().len(),
            non_declaration_callable_fragments: template
                .index
                .files()
                .iter()
                .map(|file| file.non_declaration_callable_fragments)
                .sum(),
            ..IndexBuildCounts::default()
        },
        by_source_kind: BTreeMap::new(),
        timings: IndexBuildTimings::default(),
    };

    let mut writes = Vec::with_capacity(args.iterations);
    for _ in 0..args.iterations {
        if temp_cache_path.is_file() {
            fs::remove_file(&temp_cache_path).map_err(|error| {
                format!(
                    "Failed to remove prior benchmark cache {}: {error}",
                    temp_cache_path.display()
                )
            })?;
        }
        let index = template.index.clone();
        let summary = build_summary.clone();
        let line_starts = source_line_starts.clone();
        let result = load_or_build_archive_index(
            &temp_cache_path,
            template.fingerprint.clone(),
            template.source_digest.clone(),
            move || {
                Ok(IndexBuildResult {
                    index,
                    index_shape: IndexBuildShape::RuntimeCache,
                    summary,
                    source_line_starts: line_starts,
                })
            },
        )?;
        writes.push(WriteMeasurement {
            total: result.timings.total,
            prepare: result.timings.cache_prepare,
            compact: result.timings.cache_compact,
            payload_prepare: result.timings.cache_payload_prepare,
            encode_and_write: result.timings.cache_write,
            encode: result.timings.cache_encode,
            atomic_write: result.timings.cache_atomic_write,
            bytes: result.cache_file_bytes.unwrap_or(0),
        });
    }

    if temp_cache_path.is_file() {
        fs::remove_file(&temp_cache_path).map_err(|error| {
            format!(
                "Failed to remove benchmark cache {}: {error}",
                temp_cache_path.display()
            )
        })?;
    }
    fs::remove_dir(&temp_root).map_err(|error| {
        format!(
            "Failed to remove benchmark folder {}: {error}",
            temp_root.display()
        )
    })?;

    print_results(&args, &first, &loads, &writes);
    enforce_thresholds(&args, &loads, &writes)
}

fn parse_args() -> Result<Args, String> {
    let mut values = env::args().skip(1);
    let mut cache_path = None;
    let mut iterations = DEFAULT_ITERATIONS;
    let mut max_median_load_ms = None;
    let mut max_median_write_ms = None;
    while let Some(value) = values.next() {
        match value.as_str() {
            "--cache" => {
                cache_path = Some(PathBuf::from(
                    values
                        .next()
                        .ok_or_else(|| "--cache requires a path".to_string())?,
                ));
            }
            "--iterations" => {
                iterations = parse_positive_usize(
                    "--iterations",
                    &values
                        .next()
                        .ok_or_else(|| "--iterations requires a value".to_string())?,
                )?;
            }
            "--max-median-load-ms" => {
                max_median_load_ms = Some(parse_nonnegative_f64(
                    "--max-median-load-ms",
                    &values
                        .next()
                        .ok_or_else(|| "--max-median-load-ms requires a value".to_string())?,
                )?);
            }
            "--max-median-write-ms" => {
                max_median_write_ms = Some(parse_nonnegative_f64(
                    "--max-median-write-ms",
                    &values
                        .next()
                        .ok_or_else(|| "--max-median-write-ms requires a value".to_string())?,
                )?);
            }
            "--help" | "-h" => {
                println!(
                    "Usage: cargo run --release --manifest-path server/Cargo.toml --example index_cache_io_benchmark -- --cache <symbols.bin> [--iterations <n>] [--max-median-load-ms <n>] [--max-median-write-ms <n>]"
                );
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown argument: {value}")),
        }
    }
    Ok(Args {
        cache_path: cache_path.ok_or_else(|| "--cache is required".to_string())?,
        iterations,
        max_median_load_ms,
        max_median_write_ms,
    })
}

fn load_once(cache_path: &Path) -> Result<LoadMeasurement, String> {
    let result = load_game_data_index_cache_with_control(cache_path, &Default::default())?
        .ok_or_else(|| {
            format!(
                "Cache is not compatible with the current language server: {}",
                cache_path.display()
            )
        })?;
    Ok(LoadMeasurement {
        total: result.timings.total,
        file_read: result.timings.cache_file_read,
        decode: result.timings.cache_decode,
        validate: result.timings.cache_validate,
        map_rebuild: result.timings.map_rebuild,
        projection: result.timings.map_projection,
        lookup_maps: result.timings.map_lookup_rebuild,
    })
}

fn ordered_line_starts(
    line_starts: &BTreeMap<SourceFileId, Vec<usize>>,
) -> Result<Vec<Vec<usize>>, String> {
    let mut ordered = Vec::with_capacity(line_starts.len());
    for expected in 0..line_starts.len() {
        let id = SourceFileId(expected);
        ordered.push(
            line_starts
                .get(&id)
                .cloned()
                .ok_or_else(|| format!("Cache line-start map is missing file id {expected}"))?,
        );
    }
    Ok(ordered)
}

fn print_results(
    args: &Args,
    first: &LoadMeasurement,
    loads: &[LoadMeasurement],
    writes: &[WriteMeasurement],
) {
    println!("Index cache I/O benchmark");
    println!("cache: {}", args.cache_path.display());
    println!("iterations: {}", args.iterations);
    println!(
        "first load: {:.3} ms (read {:.3}, decode {:.3}, validate {:.3}, runtime {:.3}: projection {:.3}, lookup maps {:.3})",
        millis(first.total),
        millis(first.file_read),
        millis(first.decode),
        millis(first.validate),
        millis(first.map_rebuild),
        millis(first.projection),
        millis(first.lookup_maps),
    );
    print_load_stat("median warm load", loads, percentile_index(loads.len(), 50));
    print_load_stat("p95 warm load", loads, percentile_index(loads.len(), 95));
    print_write_stat(
        "median cold cache write",
        writes,
        percentile_index(writes.len(), 50),
    );
    print_write_stat(
        "p95 cold cache write",
        writes,
        percentile_index(writes.len(), 95),
    );
}

fn print_load_stat(label: &str, values: &[LoadMeasurement], index: usize) {
    let total = sorted_duration(values.iter().map(|value| value.total), index);
    let read = sorted_duration(values.iter().map(|value| value.file_read), index);
    let decode = sorted_duration(values.iter().map(|value| value.decode), index);
    let validate = sorted_duration(values.iter().map(|value| value.validate), index);
    let maps = sorted_duration(values.iter().map(|value| value.map_rebuild), index);
    let projection = sorted_duration(values.iter().map(|value| value.projection), index);
    let lookup_maps = sorted_duration(values.iter().map(|value| value.lookup_maps), index);
    println!(
        "{label}: {:.3} ms (read {:.3}, decode {:.3}, validate {:.3}, runtime {:.3}: projection {:.3}, lookup maps {:.3})",
        millis(total),
        millis(read),
        millis(decode),
        millis(validate),
        millis(maps),
        millis(projection),
        millis(lookup_maps),
    );
}

fn print_write_stat(label: &str, values: &[WriteMeasurement], index: usize) {
    let total = sorted_duration(values.iter().map(|value| value.total), index);
    let prepare = sorted_duration(values.iter().map(|value| value.prepare), index);
    let compact = sorted_duration(values.iter().map(|value| value.compact), index);
    let payload_prepare = sorted_duration(values.iter().map(|value| value.payload_prepare), index);
    let write = sorted_duration(values.iter().map(|value| value.encode_and_write), index);
    let encode = sorted_duration(values.iter().map(|value| value.encode), index);
    let atomic_write = sorted_duration(values.iter().map(|value| value.atomic_write), index);
    let bytes = values.first().map(|value| value.bytes).unwrap_or(0);
    println!(
        "{label}: {:.3} ms (prepare {:.3}: compact {:.3}, payload {:.3}; write {:.3}: encode {:.3}, atomic {:.3}), {} bytes",
        millis(total),
        millis(prepare),
        millis(compact),
        millis(payload_prepare),
        millis(write),
        millis(encode),
        millis(atomic_write),
        bytes,
    );
}

fn enforce_thresholds(
    args: &Args,
    loads: &[LoadMeasurement],
    writes: &[WriteMeasurement],
) -> Result<(), String> {
    let median_index = percentile_index(args.iterations, 50);
    let median_load = millis(sorted_duration(
        loads.iter().map(|value| value.total),
        median_index,
    ));
    let median_write = millis(sorted_duration(
        writes.iter().map(|value| value.encode_and_write),
        median_index,
    ));
    if let Some(limit) = args.max_median_load_ms {
        if median_load > limit {
            return Err(format!(
                "Median warm load {:.3} ms exceeds limit {:.3} ms",
                median_load, limit
            ));
        }
    }
    if let Some(limit) = args.max_median_write_ms {
        if median_write > limit {
            return Err(format!(
                "Median cold cache write {:.3} ms exceeds limit {:.3} ms",
                median_write, limit
            ));
        }
    }
    Ok(())
}

fn sorted_duration(values: impl Iterator<Item = Duration>, index: usize) -> Duration {
    let mut values: Vec<_> = values.collect();
    values.sort_unstable();
    values[index.min(values.len().saturating_sub(1))]
}

fn percentile_index(len: usize, percentile: usize) -> usize {
    len.saturating_mul(percentile)
        .saturating_add(99)
        .saturating_div(100)
        .saturating_sub(1)
        .min(len.saturating_sub(1))
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn parse_positive_usize(label: &str, value: &str) -> Result<usize, String> {
    let value = value
        .parse::<usize>()
        .map_err(|error| format!("{label} must be a positive integer: {error}"))?;
    if value == 0 {
        return Err(format!("{label} must be greater than zero"));
    }
    Ok(value)
}

fn parse_nonnegative_f64(label: &str, value: &str) -> Result<f64, String> {
    let value = value
        .parse::<f64>()
        .map_err(|error| format!("{label} must be a number: {error}"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(format!("{label} must be a finite nonnegative number"));
    }
    Ok(value)
}

fn unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}
