use reforger_language_server::lsp::{run_stdio, LspServerOptions};
use std::env;
use std::path::PathBuf;

fn main() {
    if let Err(error) = run_stdio(parse_args()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn parse_args() -> LspServerOptions {
    let mut args = env::args().skip(1);
    let mut options = LspServerOptions::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--log" => {
                if let Some(value) = args.next() {
                    options.log_path = Some(PathBuf::from(value));
                }
            }
            "--game-data-scripts" => {
                if let Some(value) = args.next() {
                    options.game_data_scripts = Some(PathBuf::from(value));
                }
            }
            "--game-data-metadata" => {
                if let Some(value) = args.next() {
                    options.game_data_metadata = Some(PathBuf::from(value));
                }
            }
            "--index-cache" => {
                if let Some(value) = args.next() {
                    options.index_cache = Some(PathBuf::from(value));
                }
            }
            "--help" | "-h" => {
                println!(
                    "Usage: reforger_language_server [--log <path>] [--game-data-scripts <path>] [--game-data-metadata <path>] [--index-cache <path>]"
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }

    options
}
