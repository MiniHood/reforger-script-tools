// Shared Enfusion language and evidence modules. These do not own a client
// protocol and may be used by either adapter.
pub mod addon_sources;
pub mod analysis_runtime;
pub mod ast;
pub mod callable;
pub mod construction;
pub mod expression_type;
pub mod formatting;
pub mod game_data_catalogue;
pub mod game_data_inspection;
pub mod game_data_research;
pub mod game_data_search;
pub mod index;
pub mod index_build;
pub mod index_cache;
pub mod index_query;
pub mod lexer;
// Protocol adapters. The executable composition root selects exactly one mode.
pub mod lsp;
pub mod mcp;
pub mod model;
pub mod official_wiki;
pub mod pack;
pub mod parser;
pub mod preprocessor;
pub mod reference_finder;
pub mod resolver;
pub mod scope;
pub mod semantic_file;
pub mod symbol_display;
pub mod syntax;
pub mod type_facts;
pub mod workbench;
pub mod workbench_capture;
mod workbench_bridge;
