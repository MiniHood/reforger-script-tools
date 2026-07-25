use reforger_language_server::game_data_search::{search, GameDataSearchRequest};
use reforger_language_server::index_build::{build_index, IndexBuildConfig, IndexSourceRoot};
use reforger_language_server::model::{SourceKind, SOURCE_PRIORITY_GAME_DATA};
use std::fs;
use std::path::PathBuf;

#[test]
fn search_ranks_exact_names_before_other_semantic_matches() {
    let fixture = TempFixture::new("search-ranking");
    let scripts = fixture.path.join("Game");
    fs::create_dir_all(&scripts).expect("create scripts");
    fs::write(
        scripts.join("Search.c"),
        "class SearchTarget {}\nclass PrefixSearchTarget {}\nclass Typed { SearchTarget m_Value; }\n",
    )
    .expect("write source");
    let index = build_index(&IndexBuildConfig {
        roots: vec![IndexSourceRoot::new(
            &fixture.path,
            SourceKind::GameData,
            SOURCE_PRIORITY_GAME_DATA,
        )],
    })
    .expect("index")
    .index;

    let page = search(
        &index,
        "gd1:test",
        GameDataSearchRequest::new("SearchTarget"),
    )
    .expect("search succeeds");

    assert_eq!(page.total, 3);
    assert_eq!(page.results[0].name, "SearchTarget");
    assert_eq!(page.results[0].match_kind, "exactName");
    assert_eq!(page.results[1].name, "PrefixSearchTarget");
    assert_eq!(page.results[1].match_kind, "qualifiedName");
    assert_eq!(page.results[2].name, "m_Value");
    assert_eq!(page.results[2].match_kind, "type");
}

#[test]
fn search_pages_a_canonical_filtered_result_set_with_a_bound_cursor() {
    let fixture = TempFixture::new("search-paging");
    let scripts = fixture.path.join("Game");
    fs::create_dir_all(&scripts).expect("create scripts");
    fs::write(
        scripts.join("Paging.c"),
        "class Alpha {\n\tvoid Run(int amount) {}\n}\nclass Beta {\n\tvoid Run(string label) {}\n}\n",
    )
    .expect("write source");
    let index = build_index(&IndexBuildConfig {
        roots: vec![IndexSourceRoot::new(&fixture.path, SourceKind::GameData, SOURCE_PRIORITY_GAME_DATA)],
    })
    .expect("index")
    .index;
    let mut request = GameDataSearchRequest::new("Run");
    request.kinds = Some(vec!["method".to_string()]);
    request.source_categories = Some(vec!["game".to_string()]);
    request.limit = Some(1);
    let first = search(&index, "gd1:test", request.clone()).expect("first page");
    let cursor = first.next_cursor.clone().expect("next cursor");
    request.cursor = Some(cursor);
    let second = search(&index, "gd1:test", request).expect("second page");

    assert_eq!(first.total, 2);
    assert_eq!(first.results[0].qualified_name, "Alpha.Run");
    assert_eq!(second.results[0].qualified_name, "Beta.Run");
    assert_eq!(first.results[0].declaration_range.start_line, 2);
    assert_eq!(first.results[0].read_source_input.start_line, 2);
    assert_eq!(first.results[0].match_kind, "exactName");
}

struct TempFixture {
    path: PathBuf,
}

impl TempFixture {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "reforger-script-tools-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create fixture");
        Self { path }
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
