use reforger_language_server::game_data_search::{search, GameDataSearchRequest, SourceLineStarts};
use reforger_language_server::index::SourceFileId;
use reforger_language_server::index_build::{
    build_index, IndexBuildConfig, IndexBuildControl, IndexSourceRoot,
};
use reforger_language_server::model::{SourceKind, SOURCE_PRIORITY_GAME_DATA};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_DECLARATION_PAIRS: usize = 5_000;
const DEFAULT_ITERATIONS: usize = 31;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let declaration_pairs = argument("--declaration-pairs")?.unwrap_or(DEFAULT_DECLARATION_PAIRS);
    let iterations = argument("--iterations")?.unwrap_or(DEFAULT_ITERATIONS);
    if declaration_pairs == 0 || iterations == 0 {
        return Err("declaration pairs and iterations must be positive".into());
    }

    let fixture = Fixture::new()?;
    let scripts = fixture.path.join("Game");
    fs::create_dir_all(&scripts)?;
    let mut originals = String::new();
    let mut overlays = String::new();
    for index in 0..declaration_pairs {
        originals.push_str(&format!(
            "class Common{index:05} {{ int SharedField; void SharedMethod(); }}\n"
        ));
        overlays.push_str(&format!(
            "modded class Common{index:05} {{ int SharedField; override void SharedMethod(); }}\n"
        ));
    }
    fs::write(scripts.join("Original.c"), originals)?;
    fs::write(scripts.join("Modded.c"), overlays)?;

    let index = build_index(&IndexBuildConfig {
        roots: vec![IndexSourceRoot::new(
            &fixture.path,
            SourceKind::GameData,
            SOURCE_PRIORITY_GAME_DATA,
        )],
    })?
    .index;
    let line_starts = index
        .files()
        .iter()
        .map(|file| (file.id, SourceLineStarts::default()))
        .collect::<BTreeMap<SourceFileId, SourceLineStarts>>();
    let control = IndexBuildControl::default();
    let run = || {
        search(
            &index,
            &line_starts,
            &control,
            "semantic-search-benchmark",
            GameDataSearchRequest::new("SharedField"),
        )
    };

    let warm = run().map_err(|error| format!("semantic search failed: {error}"))?;
    let fingerprint = result_fingerprint(&warm);
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        let page = run().map_err(|error| format!("semantic search failed: {error}"))?;
        samples.push(started.elapsed());
        if result_fingerprint(&page) != fingerprint {
            return Err("semantic-search result fingerprint changed between iterations".into());
        }
    }
    samples.sort_unstable();

    println!("declarationPairs={declaration_pairs}");
    println!("matchedDeclarations={}", warm.total);
    println!("iterations={iterations}");
    println!("fingerprint={fingerprint}");
    println!("minUs={}", micros(samples[0]));
    println!("medianUs={}", micros(samples[samples.len() / 2]));
    println!(
        "p95Us={}",
        micros(samples[percentile_index(samples.len(), 95)])
    );
    println!(
        "maxUs={}",
        micros(*samples.last().expect("samples are non-empty"))
    );
    Ok(())
}

fn argument(name: &str) -> Result<Option<usize>, Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    let Some(position) = args.iter().position(|argument| argument == name) else {
        return Ok(None);
    };
    let value = args
        .get(position + 1)
        .ok_or_else(|| format!("{name} requires a value"))?;
    Ok(Some(value.parse()?))
}

fn result_fingerprint(
    page: &reforger_language_server::game_data_search::GameDataSearchPage,
) -> String {
    page.results
        .iter()
        .map(|result| format!("{}:{}", result.qualified_name, result.relative_path))
        .collect::<Vec<_>>()
        .join("|")
}

fn percentile_index(len: usize, percentile: usize) -> usize {
    ((len - 1) * percentile).div_ceil(100)
}

fn micros(duration: Duration) -> u128 {
    duration.as_micros()
}

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = env::temp_dir().join(format!(
            "reforger-semantic-search-benchmark-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
