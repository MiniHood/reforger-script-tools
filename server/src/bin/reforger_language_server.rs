use reforger_language_server::game_data_catalogue::GameDataCatalogueConfig;
use reforger_language_server::lsp::{
    run_stdio as run_lsp_stdio, BracketColoringMode, LspServerOptions,
};
use reforger_language_server::mcp::{
    render_api_reference, run_stdio as run_mcp_stdio, McpServerOptions,
};
use std::env;
use std::path::PathBuf;

enum ServerMode {
    Lsp(LspServerOptions),
    Mcp(McpServerOptions),
    McpApi,
    Help,
}

fn main() {
    let mode = match parse_args() {
        Ok(mode) => mode,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };

    let result = match mode {
        ServerMode::Lsp(options) => run_lsp_stdio(options),
        ServerMode::Mcp(options) => run_mcp_stdio(options),
        ServerMode::McpApi => {
            print!("{}", render_api_reference());
            Ok(())
        }
        ServerMode::Help => {
            print_help();
            Ok(())
        }
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn parse_args() -> Result<ServerMode, String> {
    parse_args_from(env::args().skip(1))
}

fn parse_args_from(args: impl Iterator<Item = String>) -> Result<ServerMode, String> {
    let mut args = args.peekable();
    match args.peek().map(String::as_str) {
        Some("mcp") => {
            args.next();
            parse_mcp_args(args).map(ServerMode::Mcp)
        }
        Some("mcp-api") => {
            args.next();
            if let Some(argument) = args.next() {
                Err(format!("unexpected argument for mcp-api mode: {argument}"))
            } else {
                Ok(ServerMode::McpApi)
            }
        }
        Some("--help" | "-h") => {
            args.next();
            if let Some(argument) = args.next() {
                Err(format!("unexpected argument after help flag: {argument}"))
            } else {
                Ok(ServerMode::Help)
            }
        }
        Some(argument) if !argument.starts_with('-') => Err(format!("unknown mode '{argument}'")),
        _ => parse_lsp_args(args).map(ServerMode::Lsp),
    }
}

fn parse_lsp_args(mut args: impl Iterator<Item = String>) -> Result<LspServerOptions, String> {
    let mut options = LspServerOptions::default();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            // vscode-languageclient appends this marker for TransportKind.stdio.
            // The process already owns stdio; accepting the known marker keeps
            // the legacy LSP launch contract strict without changing transport.
            "--stdio" => {}
            "--log" => options.log_path = Some(path_value(&mut args, "--log")?),
            "--diagnostic-log" => {
                options.diagnostic_log_path = Some(path_value(&mut args, "--diagnostic-log")?)
            }
            "--game-data-scripts" => {
                options.game_data_scripts = Some(path_value(&mut args, "--game-data-scripts")?)
            }
            "--game-data-metadata" => {
                options.game_data_metadata = Some(path_value(&mut args, "--game-data-metadata")?)
            }
            "--index-cache" => options.index_cache = Some(path_value(&mut args, "--index-cache")?),
            "--workspace-scripts" => {
                options
                    .workspace_scripts
                    .push(path_value(&mut args, "--workspace-scripts")?);
            }
            "--bracket-coloring" => {
                let value = string_value(&mut args, "--bracket-coloring")?;
                options.bracket_coloring = match value.as_str() {
                    "semantic" => BracketColoringMode::Semantic,
                    "punctuation" => BracketColoringMode::Punctuation,
                    "vscode" => BracketColoringMode::VsCode,
                    _ => {
                        return Err(format!("invalid value for --bracket-coloring: {value}"));
                    }
                };
            }
            _ => return Err(format!("unknown LSP argument '{argument}'")),
        }
    }

    Ok(options)
}

fn parse_mcp_args(mut args: impl Iterator<Item = String>) -> Result<McpServerOptions, String> {
    let mut game_data = GameDataCatalogueConfig {
        scripts_root: None,
        metadata_path: None,
        cache_path: None,
    };

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--game-data-scripts" => {
                game_data.scripts_root = Some(path_value(&mut args, "--game-data-scripts")?)
            }
            "--game-data-metadata" => {
                game_data.metadata_path = Some(path_value(&mut args, "--game-data-metadata")?)
            }
            "--index-cache" => game_data.cache_path = Some(path_value(&mut args, "--index-cache")?),
            _ => return Err(format!("unknown MCP argument '{argument}'")),
        }
    }

    Ok(McpServerOptions { game_data })
}

fn path_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<PathBuf, String> {
    string_value(args, flag).map(PathBuf::from)
}

fn string_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    let value = args
        .next()
        .ok_or_else(|| format!("missing value for {flag}"))?;
    if value.is_empty() || value.starts_with("--") {
        return Err(format!("missing value for {flag}"));
    }
    Ok(value)
}

fn print_help() {
    println!(
        "Usage:\n  reforger_language_server [LSP options]\n  reforger_language_server mcp [MCP options]\n  reforger_language_server mcp-api\n\nLSP options:\n  --log <path>\n  --diagnostic-log <path>\n  --game-data-scripts <path>\n  --game-data-metadata <path>\n  --index-cache <path>\n  --workspace-scripts <path> (repeatable)\n  --bracket-coloring <semantic|punctuation|vscode>\n\nMCP options:\n  --game-data-scripts <path>\n  --game-data-metadata <path>\n  --index-cache <path>"
    );
}

#[cfg(test)]
mod tests {
    use super::{parse_args_from, ServerMode};
    use reforger_language_server::lsp::BracketColoringMode;
    use std::path::PathBuf;

    #[test]
    fn bracket_coloring_argument_accepts_every_extension_setting_value() {
        for (value, expected) in [
            ("semantic", BracketColoringMode::Semantic),
            ("punctuation", BracketColoringMode::Punctuation),
            ("vscode", BracketColoringMode::VsCode),
        ] {
            let mode =
                parse_args_from(["--bracket-coloring".to_string(), value.to_string()].into_iter())
                    .expect("valid LSP arguments");
            let ServerMode::Lsp(options) = mode else {
                panic!("expected LSP mode");
            };
            assert_eq!(options.bracket_coloring, expected);
        }
    }

    #[test]
    fn explicit_mcp_mode_is_separate_from_legacy_lsp_mode() {
        let mode = parse_args_from(
            [
                "mcp".to_string(),
                "--game-data-scripts".to_string(),
                "scripts".to_string(),
                "--index-cache".to_string(),
                "cache.bin".to_string(),
            ]
            .into_iter(),
        )
        .expect("valid MCP arguments");

        let ServerMode::Mcp(options) = mode else {
            panic!("expected MCP mode");
        };
        assert_eq!(
            options.game_data.scripts_root,
            Some(PathBuf::from("scripts"))
        );
        assert_eq!(
            options.game_data.cache_path,
            Some(PathBuf::from("cache.bin"))
        );
    }

    #[test]
    fn unknown_modes_flags_and_missing_values_are_rejected() {
        for arguments in [
            vec!["unknown-mode".to_string()],
            vec!["--unknown".to_string()],
            vec!["--log".to_string()],
            vec!["mcp".to_string(), "--index-cache".to_string()],
        ] {
            assert!(parse_args_from(arguments.into_iter()).is_err());
        }
    }

    #[test]
    fn lsp_accepts_the_language_client_stdio_transport_marker() {
        let mode = parse_args_from(["--stdio".to_string()].into_iter())
            .expect("known language-client transport marker");
        assert!(matches!(mode, ServerMode::Lsp(_)));
    }
}
