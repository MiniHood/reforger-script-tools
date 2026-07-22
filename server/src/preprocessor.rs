use crate::lexer::TextSpan;

/// The complete directive vocabulary observed in verified Reforger game data.
/// Parser support remains broader for recovery, but completion advertises only
/// this evidence-backed authoring surface.
pub const COMPLETION_DIRECTIVES: [&str; 5] = ["define", "ifdef", "ifndef", "else", "endif"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionContext {
    Directive { prefix: String, span: TextSpan },
    Macro { prefix: String, span: TextSpan },
}

/// Classifies the editor's incomplete directive shape without assigning any
/// preprocessor semantics. The caller supplies lexical source eligibility
/// (comments and strings) from the shared lexer.
pub fn completion_context_at_offset(source: &str, offset: usize) -> Option<CompletionContext> {
    let line_start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
    let line = source.get(line_start..offset)?;
    let hash_offset = line_start + line.find(|character: char| !character.is_whitespace())?;
    if source.as_bytes().get(hash_offset) != Some(&b'#') || offset <= hash_offset {
        return None;
    }
    let after_hash = source.get(hash_offset + 1..offset)?;
    if !after_hash.contains(char::is_whitespace) {
        return Some(CompletionContext::Directive {
            prefix: after_hash.to_string(),
            span: TextSpan::new(hash_offset + 1, offset),
        });
    }
    let directive_end = after_hash.find(char::is_whitespace)?;
    if !matches!(&after_hash[..directive_end], "ifdef" | "ifndef") {
        return None;
    }
    let operand_start = after_hash[directive_end..]
        .find(|character: char| !character.is_whitespace())
        .map(|index| hash_offset + 1 + directive_end + index)
        .unwrap_or(offset);
    let operand = source.get(operand_start..offset)?;
    (!operand.chars().any(char::is_whitespace)).then_some(CompletionContext::Macro {
        prefix: operand.to_string(),
        span: TextSpan::new(operand_start, offset),
    })
}
