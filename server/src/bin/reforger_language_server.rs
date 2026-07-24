use reforger_language_server::lsp::{run_stdio, BracketColoringMode, LspServerOptions};
use std::env;
use std::path::PathBuf;

fn main() {
    if let Err(error) = run_stdio(parse_args()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn parse_args() -> LspServerOptions {
    parse_args_from(env::args().skip(1))
}

fn parse_args_from(mut args: impl Iterator<Item = String>) -> LspServerOptions {
    let mut options = LspServerOptions::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--log" => {
                if let Some(value) = args.next() {
                    options.log_path = Some(PathBuf::from(value));
                }
            }
            "--diagnostic-log" => {
                if let Some(value) = args.next() {
                    options.diagnostic_log_path = Some(PathBuf::from(value));
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
            "--workspace-scripts" => {
                if let Some(value) = args.next() {
                    options.workspace_scripts.push(PathBuf::from(value));
                }
            }
            "--bracket-coloring" => {
                if let Some(value) = args.next() {
                    options.bracket_coloring = match value.as_str() {
                        "punctuation" => BracketColoringMode::Punctuation,
                        "vscode" => BracketColoringMode::VsCode,
                        _ => BracketColoringMode::Semantic,
                    };
                }
            }
            "--help" | "-h" => {
                println!(
                    "Usage: reforger_language_server [--log <path>] [--diagnostic-log <path>] [--game-data-scripts <path>] [--game-data-metadata <path>] [--index-cache <path>] [--workspace-scripts <path>]... [--bracket-coloring <semantic|punctuation|vscode>]"
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }

    options
}

#[cfg(test)]
mod tests {
    use super::parse_args_from;
    use reforger_language_server::lsp::BracketColoringMode;

    #[test]
    fn bracket_coloring_argument_accepts_every_extension_setting_value() {
        for (value, expected) in [
            ("semantic", BracketColoringMode::Semantic),
            ("punctuation", BracketColoringMode::Punctuation),
            ("vscode", BracketColoringMode::VsCode),
        ] {
            let options =
                parse_args_from(["--bracket-coloring".to_string(), value.to_string()].into_iter());
            assert_eq!(options.bracket_coloring, expected);
        }
    }
}
