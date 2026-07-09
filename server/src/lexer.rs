#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
}

impl TextSpan {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn len(self) -> usize {
        self.end - self.start
    }

    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: TextSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Whitespace,
    LineComment,
    DocLineComment,
    BlockComment,
    DocBlockComment,
    UnterminatedBlockComment,
    Identifier,
    Keyword(Keyword),
    Number,
    InvalidNumber,
    String,
    UnterminatedString,
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Semicolon,
    Colon,
    Comma,
    Dot,
    Question,
    Hash,
    Operator(Operator),
    Unknown,
    Eof,
}

impl TokenKind {
    pub const fn is_trivia(self) -> bool {
        matches!(
            self,
            Self::Whitespace
                | Self::LineComment
                | Self::DocLineComment
                | Self::BlockComment
                | Self::DocBlockComment
        )
    }

    pub const fn is_error(self) -> bool {
        matches!(
            self,
            Self::UnterminatedBlockComment
                | Self::InvalidNumber
                | Self::UnterminatedString
                | Self::Unknown
        )
    }

    pub const fn is_keyword(self) -> bool {
        matches!(self, Self::Keyword(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Class,
    Modded,
    Sealed,
    Extends,
    Typedef,
    Func,
    Proto,
    External,
    Native,
    Volatile,
    Private,
    Protected,
    Static,
    Override,
    Const,
    Ref,
    Out,
    Inout,
    Notnull,
    Autoptr,
    Owned,
    Void,
    Int,
    Float,
    Bool,
    String,
    Vector,
    Typename,
    Enum,
    True,
    False,
    Null,
    Auto,
    New,
    Delete,
    Thread,
    If,
    Else,
    For,
    Foreach,
    While,
    Do,
    Switch,
    Case,
    Default,
    Break,
    Continue,
    Return,
    This,
    Super,
    Vanilla,
    Debug,
    Event,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Plus,
    PlusPlus,
    PlusEqual,
    Minus,
    MinusMinus,
    MinusEqual,
    Star,
    StarEqual,
    Slash,
    SlashEqual,
    Percent,
    PercentEqual,
    Equal,
    EqualEqual,
    Bang,
    BangEqual,
    Less,
    LessEqual,
    LessLess,
    LessLessEqual,
    Greater,
    GreaterEqual,
    GreaterGreater,
    GreaterGreaterEqual,
    Ampersand,
    AmpersandAmpersand,
    AmpersandEqual,
    Pipe,
    PipePipe,
    PipeEqual,
    Caret,
    CaretEqual,
    Tilde,
    Arrow,
}

pub fn lex(source: &str) -> Vec<Token> {
    Lexer::new(source).lex_all()
}

struct Lexer<'source> {
    source: &'source str,
    position: usize,
    tokens: Vec<Token>,
}

impl<'source> Lexer<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            source,
            position: 0,
            tokens: Vec::new(),
        }
    }

    fn lex_all(mut self) -> Vec<Token> {
        while !self.is_at_end() {
            self.lex_token();
        }

        self.push(TokenKind::Eof, self.position, self.position);
        self.tokens
    }

    fn lex_token(&mut self) {
        let start = self.position;
        let Some(current) = self.peek_char() else {
            return;
        };

        match current {
            c if c.is_ascii_whitespace() => self.lex_whitespace(start),
            c if is_identifier_start(c) => self.lex_identifier_or_keyword(start),
            c if c.is_ascii_digit() => self.lex_number(start),
            '"' => self.lex_string(start),
            '/' => self.lex_slash_or_comment(start),
            '{' => self.single_char(TokenKind::LeftBrace, start),
            '}' => self.single_char(TokenKind::RightBrace, start),
            '(' => self.single_char(TokenKind::LeftParen, start),
            ')' => self.single_char(TokenKind::RightParen, start),
            '[' => self.single_char(TokenKind::LeftBracket, start),
            ']' => self.single_char(TokenKind::RightBracket, start),
            ';' => self.single_char(TokenKind::Semicolon, start),
            ':' => self.single_char(TokenKind::Colon, start),
            ',' => self.single_char(TokenKind::Comma, start),
            '.' => self.single_char(TokenKind::Dot, start),
            '?' => self.single_char(TokenKind::Question, start),
            '#' => self.single_char(TokenKind::Hash, start),
            '+' => self.lex_operator(
                start,
                &[("++", Operator::PlusPlus), ("+=", Operator::PlusEqual)],
                Operator::Plus,
            ),
            '-' => self.lex_operator(
                start,
                &[
                    ("->", Operator::Arrow),
                    ("--", Operator::MinusMinus),
                    ("-=", Operator::MinusEqual),
                ],
                Operator::Minus,
            ),
            '*' => self.lex_operator(start, &[("*=", Operator::StarEqual)], Operator::Star),
            '%' => self.lex_operator(start, &[("%=", Operator::PercentEqual)], Operator::Percent),
            '=' => self.lex_operator(start, &[("==", Operator::EqualEqual)], Operator::Equal),
            '!' => self.lex_operator(start, &[("!=", Operator::BangEqual)], Operator::Bang),
            '<' => self.lex_operator(
                start,
                &[
                    ("<<=", Operator::LessLessEqual),
                    ("<<", Operator::LessLess),
                    ("<=", Operator::LessEqual),
                ],
                Operator::Less,
            ),
            '>' => self.lex_operator(
                start,
                &[
                    (">>=", Operator::GreaterGreaterEqual),
                    (">>", Operator::GreaterGreater),
                    (">=", Operator::GreaterEqual),
                ],
                Operator::Greater,
            ),
            '&' => self.lex_operator(
                start,
                &[
                    ("&&", Operator::AmpersandAmpersand),
                    ("&=", Operator::AmpersandEqual),
                ],
                Operator::Ampersand,
            ),
            '|' => self.lex_operator(
                start,
                &[("||", Operator::PipePipe), ("|=", Operator::PipeEqual)],
                Operator::Pipe,
            ),
            '^' => self.lex_operator(start, &[("^=", Operator::CaretEqual)], Operator::Caret),
            '~' => self.single_operator(Operator::Tilde, start),
            _ => {
                self.advance_char();
                self.push(TokenKind::Unknown, start, self.position);
            }
        }
    }

    fn lex_whitespace(&mut self, start: usize) {
        self.advance_char();
        while matches!(self.peek_char(), Some(c) if c.is_ascii_whitespace()) {
            self.advance_char();
        }
        self.push(TokenKind::Whitespace, start, self.position);
    }

    fn lex_identifier_or_keyword(&mut self, start: usize) {
        self.advance_char();
        while matches!(self.peek_char(), Some(c) if is_identifier_continue(c)) {
            self.advance_char();
        }

        let text = &self.source[start..self.position];
        let kind = match keyword_from_str(text) {
            Some(keyword) => TokenKind::Keyword(keyword),
            None => TokenKind::Identifier,
        };
        self.push(kind, start, self.position);
    }

    fn lex_number(&mut self, start: usize) {
        self.advance_char();

        if self.source[start..].starts_with("0x") || self.source[start..].starts_with("0X") {
            self.advance_char();
            let hex_start = self.position;
            while matches!(self.peek_char(), Some(c) if c.is_ascii_hexdigit()) {
                self.advance_char();
            }
            let kind = if self.position == hex_start {
                while matches!(self.peek_char(), Some(c) if is_identifier_continue(c)) {
                    self.advance_char();
                }
                TokenKind::InvalidNumber
            } else {
                TokenKind::Number
            };
            self.push(kind, start, self.position);
            return;
        }

        while matches!(self.peek_char(), Some(c) if c.is_ascii_digit()) {
            self.advance_char();
        }

        if self.peek_char() == Some('.')
            && self.peek_next_char().is_some_and(|c| c.is_ascii_digit())
        {
            self.advance_char();
            while matches!(self.peek_char(), Some(c) if c.is_ascii_digit()) {
                self.advance_char();
            }
        }

        if matches!(self.peek_char(), Some('e' | 'E')) {
            self.advance_char();
            if matches!(self.peek_char(), Some('+' | '-')) {
                self.advance_char();
            }

            if matches!(self.peek_char(), Some(c) if c.is_ascii_digit()) {
                while matches!(self.peek_char(), Some(c) if c.is_ascii_digit()) {
                    self.advance_char();
                }
            } else {
                while matches!(self.peek_char(), Some(c) if is_identifier_continue(c)) {
                    self.advance_char();
                }
                self.push(TokenKind::InvalidNumber, start, self.position);
                return;
            }
        }

        self.push(TokenKind::Number, start, self.position);
    }

    fn lex_string(&mut self, start: usize) {
        self.advance_char();
        let mut escaped = false;

        while let Some(current) = self.peek_char() {
            if current == '\n' || current == '\r' {
                self.push(TokenKind::UnterminatedString, start, self.position);
                return;
            }

            self.advance_char();

            if escaped {
                escaped = false;
                continue;
            }

            if current == '\\' {
                escaped = true;
                continue;
            }

            if current == '"' {
                self.push(TokenKind::String, start, self.position);
                return;
            }
        }

        self.push(TokenKind::UnterminatedString, start, self.position);
    }

    fn lex_slash_or_comment(&mut self, start: usize) {
        if self.source[start..].starts_with("//") {
            let kind = if self.source[start..].starts_with("//!") {
                TokenKind::DocLineComment
            } else {
                TokenKind::LineComment
            };
            self.position += 2;
            while let Some(current) = self.peek_char() {
                if current == '\n' || current == '\r' {
                    break;
                }
                self.advance_char();
            }
            self.push(kind, start, self.position);
            return;
        }

        if self.source[start..].starts_with("/*") {
            let kind = if self.source[start..].starts_with("/*!") {
                TokenKind::DocBlockComment
            } else {
                TokenKind::BlockComment
            };
            self.position += 2;
            while !self.is_at_end() {
                if self.source[self.position..].starts_with("*/") {
                    self.position += 2;
                    self.push(kind, start, self.position);
                    return;
                }
                self.advance_char();
            }
            self.push(TokenKind::UnterminatedBlockComment, start, self.position);
            return;
        }

        self.lex_operator(start, &[("/=", Operator::SlashEqual)], Operator::Slash);
    }

    fn lex_operator(&mut self, start: usize, candidates: &[(&str, Operator)], fallback: Operator) {
        for (text, operator) in candidates {
            if self.source[start..].starts_with(text) {
                self.position += text.len();
                self.push(TokenKind::Operator(*operator), start, self.position);
                return;
            }
        }

        self.single_operator(fallback, start);
    }

    fn single_char(&mut self, kind: TokenKind, start: usize) {
        self.advance_char();
        self.push(kind, start, self.position);
    }

    fn single_operator(&mut self, operator: Operator, start: usize) {
        self.advance_char();
        self.push(TokenKind::Operator(operator), start, self.position);
    }

    fn push(&mut self, kind: TokenKind, start: usize, end: usize) {
        self.tokens.push(Token {
            kind,
            span: TextSpan::new(start, end),
        });
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.source.len()
    }

    fn peek_char(&self) -> Option<char> {
        self.source[self.position..].chars().next()
    }

    fn peek_next_char(&self) -> Option<char> {
        let mut chars = self.source[self.position..].chars();
        chars.next()?;
        chars.next()
    }

    fn advance_char(&mut self) -> Option<char> {
        let current = self.peek_char()?;
        self.position += current.len_utf8();
        Some(current)
    }
}

fn is_identifier_start(value: char) -> bool {
    value == '_' || value.is_ascii_alphabetic()
}

fn is_identifier_continue(value: char) -> bool {
    value == '_' || value.is_ascii_alphanumeric()
}

fn keyword_from_str(value: &str) -> Option<Keyword> {
    match value {
        "class" => Some(Keyword::Class),
        "modded" => Some(Keyword::Modded),
        "sealed" => Some(Keyword::Sealed),
        "extends" => Some(Keyword::Extends),
        "typedef" => Some(Keyword::Typedef),
        "func" => Some(Keyword::Func),
        "proto" => Some(Keyword::Proto),
        "external" => Some(Keyword::External),
        "native" => Some(Keyword::Native),
        "volatile" => Some(Keyword::Volatile),
        "private" => Some(Keyword::Private),
        "protected" => Some(Keyword::Protected),
        "static" => Some(Keyword::Static),
        "override" => Some(Keyword::Override),
        "const" => Some(Keyword::Const),
        "ref" => Some(Keyword::Ref),
        "out" => Some(Keyword::Out),
        "inout" => Some(Keyword::Inout),
        "notnull" => Some(Keyword::Notnull),
        "autoptr" => Some(Keyword::Autoptr),
        "owned" => Some(Keyword::Owned),
        "void" => Some(Keyword::Void),
        "int" => Some(Keyword::Int),
        "float" => Some(Keyword::Float),
        "bool" => Some(Keyword::Bool),
        "string" => Some(Keyword::String),
        "vector" => Some(Keyword::Vector),
        "typename" => Some(Keyword::Typename),
        "enum" => Some(Keyword::Enum),
        "true" => Some(Keyword::True),
        "false" => Some(Keyword::False),
        "null" | "NULL" => Some(Keyword::Null),
        "auto" => Some(Keyword::Auto),
        "new" => Some(Keyword::New),
        "delete" => Some(Keyword::Delete),
        "thread" => Some(Keyword::Thread),
        "if" => Some(Keyword::If),
        "else" => Some(Keyword::Else),
        "for" => Some(Keyword::For),
        "foreach" => Some(Keyword::Foreach),
        "while" => Some(Keyword::While),
        "do" => Some(Keyword::Do),
        "switch" => Some(Keyword::Switch),
        "case" => Some(Keyword::Case),
        "default" => Some(Keyword::Default),
        "break" => Some(Keyword::Break),
        "continue" => Some(Keyword::Continue),
        "return" => Some(Keyword::Return),
        "this" => Some(Keyword::This),
        "super" => Some(Keyword::Super),
        "vanilla" => Some(Keyword::Vanilla),
        "debug" => Some(Keyword::Debug),
        "event" => Some(Keyword::Event),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn non_trivia_kinds(source: &str) -> Vec<TokenKind> {
        lex(source)
            .into_iter()
            .filter_map(|token| (!token.kind.is_trivia()).then_some(token.kind))
            .collect()
    }

    #[test]
    fn lexes_class_inheritance_forms() {
        let kinds = non_trivia_kinds("class Child : Parent {}\nclass Other extends Base {}");

        assert_eq!(
            kinds,
            vec![
                TokenKind::Keyword(Keyword::Class),
                TokenKind::Identifier,
                TokenKind::Colon,
                TokenKind::Identifier,
                TokenKind::LeftBrace,
                TokenKind::RightBrace,
                TokenKind::Keyword(Keyword::Class),
                TokenKind::Identifier,
                TokenKind::Keyword(Keyword::Extends),
                TokenKind::Identifier,
                TokenKind::LeftBrace,
                TokenKind::RightBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_modded_class_attributes_and_generics() {
        let source = r#"[BaseContainerProps(configRoot: true)]
modded class PlayerNameInputController
{
    [Attribute()]
    protected ref map<typename, ref array<string>> m_mNames;
}"#;

        let kinds = non_trivia_kinds(source);

        assert!(kinds.contains(&TokenKind::Keyword(Keyword::Modded)));
        assert!(kinds.contains(&TokenKind::Keyword(Keyword::Class)));
        assert!(kinds.contains(&TokenKind::LeftBracket));
        assert!(kinds.contains(&TokenKind::RightBracket));
        assert!(kinds.contains(&TokenKind::Operator(Operator::Less)));
        assert!(kinds.contains(&TokenKind::Operator(Operator::GreaterGreater)));
    }

    #[test]
    fn lexes_comments_as_trivia() {
        let source = "//! doc\n/* block */\nclass Example {}";
        let tokens = lex(source);

        assert_eq!(tokens[0].kind, TokenKind::DocLineComment);
        assert_eq!(&source[tokens[0].span.start..tokens[0].span.end], "//! doc");
        assert!(tokens
            .iter()
            .any(|token| token.kind == TokenKind::BlockComment));
    }

    #[test]
    fn distinguishes_doc_comments_without_parsing_tags() {
        let source = "/*!\n\\param value input\n\\return output\n\\code\nPrint(value);\n\\endcode\n*/\n// normal\n//! doc line";
        let tokens = lex(source);

        assert_eq!(tokens[0].kind, TokenKind::DocBlockComment);
        assert_eq!(
            &source[tokens[0].span.start..tokens[0].span.end],
            "/*!\n\\param value input\n\\return output\n\\code\nPrint(value);\n\\endcode\n*/"
        );
        assert!(tokens
            .iter()
            .any(|token| token.kind == TokenKind::LineComment));
        assert!(tokens
            .iter()
            .any(|token| token.kind == TokenKind::DocLineComment));
    }

    #[test]
    fn lexes_method_signature_rpc_and_preprocessor_tokens() {
        let source = "#define FEATURE\n[RplRpc(RplChannel.Reliable)]\nproto external void RpcDo(int value = 10);";
        let kinds = non_trivia_kinds(source);

        assert_eq!(kinds[0], TokenKind::Hash);
        assert_eq!(kinds[1], TokenKind::Identifier);
        assert!(kinds.contains(&TokenKind::Keyword(Keyword::Proto)));
        assert!(kinds.contains(&TokenKind::Keyword(Keyword::External)));
        assert!(kinds.contains(&TokenKind::Keyword(Keyword::Void)));
        assert!(kinds.contains(&TokenKind::Number));
        assert!(kinds.contains(&TokenKind::Semicolon));
    }

    #[test]
    fn lexes_documented_script_keywords_found_in_game_data() {
        let source =
            "typedef func Callback;\nevent protected void EOnInit(IEntity owner);\nauto item = new ClassName();\nthread DelayedStart(id);\ndebug;\nvanilla.Hello();";
        let kinds = non_trivia_kinds(source);

        assert!(kinds.contains(&TokenKind::Keyword(Keyword::Typedef)));
        assert!(kinds.contains(&TokenKind::Keyword(Keyword::Func)));
        assert!(kinds.contains(&TokenKind::Keyword(Keyword::Event)));
        assert!(kinds.contains(&TokenKind::Keyword(Keyword::Auto)));
        assert!(kinds.contains(&TokenKind::Keyword(Keyword::Thread)));
        assert!(kinds.contains(&TokenKind::Keyword(Keyword::Debug)));
        assert!(kinds.contains(&TokenKind::Keyword(Keyword::Vanilla)));
    }

    #[test]
    fn lexes_hex_float_and_scientific_numbers() {
        let source = "int color = 0xFFFFFFFF; float tiny = 1.0e-8; float ratio = 20.0;";
        let tokens = lex(source);
        let number_texts: Vec<&str> = tokens
            .iter()
            .filter_map(|token| {
                if token.kind == TokenKind::Number {
                    Some(&source[token.span.start..token.span.end])
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(number_texts, vec!["0xFFFFFFFF", "1.0e-8", "20.0"]);
    }

    #[test]
    fn reports_invalid_number_tokens() {
        let invalid_hex = lex("int color = 0xG;");
        assert!(invalid_hex
            .iter()
            .any(|token| token.kind == TokenKind::InvalidNumber));

        let invalid_exponent = lex("float bad = 1e+;");
        assert!(invalid_exponent
            .iter()
            .any(|token| token.kind == TokenKind::InvalidNumber));
    }

    #[test]
    fn reports_unterminated_string_and_comment_tokens() {
        let string_tokens = lex("\"unterminated");
        assert_eq!(string_tokens[0].kind, TokenKind::UnterminatedString);

        let comment_tokens = lex("/* unterminated");
        assert_eq!(comment_tokens[0].kind, TokenKind::UnterminatedBlockComment);
    }

    #[test]
    fn lexes_committed_fixtures_without_error_tokens() {
        let fixtures = [
            include_str!("../../tools/fixtures/lexer/core_language_shapes.c"),
            include_str!("../../tools/fixtures/lexer/declarations.c"),
            include_str!("../../tools/fixtures/lexer/game_data_player_commands_config.c"),
            include_str!("../../tools/fixtures/lexer/game_editor_preview_params.c"),
            include_str!("../../tools/fixtures/lexer/core_array_class.c"),
            include_str!("../../tools/fixtures/lexer/modded_game_mode_shapes.c"),
            include_str!("../../tools/fixtures/lexer/trivia_preprocessor_rpc.c"),
            include_str!("../../tools/fixtures/lexer/workbench_basic_code_formatter_excerpt.c"),
        ];

        for fixture in fixtures {
            let tokens = lex(fixture);
            assert!(!tokens.iter().any(|token| token.kind.is_error()));
        }
    }

    #[test]
    fn lexes_parser_facing_core_shapes() {
        let source = "class func {}\nstring.ToString(item);\nclass map<Class TKey,Class TValue>: Managed {}\ntypedef map<ref Managed, ref Managed> TManagedRefManagedRefMap;\nproto int Init(T init[]);\nref map<typename, ref array<string>> m_mNames;";
        let kinds = non_trivia_kinds(source);

        assert!(kinds.contains(&TokenKind::Keyword(Keyword::Func)));
        assert!(kinds.contains(&TokenKind::Keyword(Keyword::String)));
        assert!(kinds.contains(&TokenKind::Keyword(Keyword::Typename)));
        assert!(kinds.contains(&TokenKind::Operator(Operator::GreaterGreater)));
        assert!(kinds.contains(&TokenKind::LeftBracket));
        assert!(kinds.contains(&TokenKind::RightBracket));
    }

    #[test]
    fn records_byte_spans_without_copying_text() {
        let source = "class Example";
        let tokens = lex(source);

        assert_eq!(tokens[0].span, TextSpan::new(0, 5));
        assert_eq!(&source[tokens[0].span.start..tokens[0].span.end], "class");
        assert_eq!(tokens[2].span, TextSpan::new(6, 13));
        assert_eq!(&source[tokens[2].span.start..tokens[2].span.end], "Example");
    }

    #[test]
    fn classifies_token_helpers() {
        assert!(TokenKind::Whitespace.is_trivia());
        assert!(TokenKind::DocBlockComment.is_trivia());
        assert!(TokenKind::InvalidNumber.is_error());
        assert!(TokenKind::UnterminatedString.is_error());
        assert!(TokenKind::Keyword(Keyword::Class).is_keyword());
        assert!(!TokenKind::Identifier.is_keyword());
    }
}
