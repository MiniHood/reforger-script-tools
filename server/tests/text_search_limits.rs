use reforger_language_server::index_build::IndexBuildControl;
use reforger_language_server::text_search::{
    search, TextSearchCorpus, TextSearchOptions, TextSearchRequest, TextSource,
};
use std::sync::Arc;

const MAX_SEARCH_RESULTS: usize = 10_000;

#[test]
fn broad_search_stops_after_proving_the_shared_result_limit_was_exceeded() {
    let content = "SCR_ ".repeat(MAX_SEARCH_RESULTS + 1);
    let page = search(
        TextSearchCorpus {
            files_considered: 2,
            sources: vec![
                TextSource {
                    relative_path: "Game/A.c".to_string(),
                    addon_guid: None,
                    addon_label: None,
                    thumbnail_color: None,
                    source_uri: None,
                    content: Arc::from(content),
                },
                TextSource {
                    relative_path: "Game/B.c".to_string(),
                    addon_guid: None,
                    addon_label: None,
                    thumbnail_color: None,
                    source_uri: None,
                    content: Arc::from("SCR_"),
                },
            ],
            ..TextSearchCorpus::default()
        },
        &IndexBuildControl::default(),
        "ws1:test",
        TextSearchRequest {
            query: "SCR_".to_string(),
            addon_guids: None,
            options: TextSearchOptions::default(),
            limit: Some(10),
            cursor: None,
        },
    )
    .expect("bounded broad text search");

    assert_eq!(page.total, MAX_SEARCH_RESULTS);
    assert!(page.truncated);
    assert_eq!(page.stats.matches_found, page.total + 1);
    assert_eq!(page.stats.files_read, 1);
    assert_eq!(page.stats.files_with_matches, 1);
}
