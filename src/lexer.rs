#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Fn,
    Let,
    Print,
    Worker,
    Submit,
    Wait,
    If,
    Else,
    While,
    Return,
    Break,
    Continue,
    Ident(String),
    Number(i64),
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semicolon,
    Comma,
    Dot,
    Equal,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Plus,
    Minus,
    Star,
    Slash,
    Eof,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub offset: usize,
}

pub fn lex(source: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let digits: String = chars[start..i].iter().collect();
            let value = digits.parse().map_err(|_| {
                format!("integer literal overflow at byte/character offset {start}")
            })?;
            out.push(Token {
                kind: TokenKind::Number(value),
                offset: start,
            });
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let kind = match word.as_str() {
                "fn" => TokenKind::Fn,
                "let" => TokenKind::Let,
                "print" => TokenKind::Print,
                "worker" => TokenKind::Worker,
                "submit" => TokenKind::Submit,
                "wait" => TokenKind::Wait,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "while" => TokenKind::While,
                "return" => TokenKind::Return,
                "break" => TokenKind::Break,
                "continue" => TokenKind::Continue,
                _ => TokenKind::Ident(word),
            };
            out.push(Token {
                kind,
                offset: start,
            });
            continue;
        }
        let next = chars.get(i + 1).copied();
        let (kind, width) = match (c, next) {
            ('=', Some('=')) => (TokenKind::EqualEqual, 2),
            ('!', Some('=')) => (TokenKind::BangEqual, 2),
            ('<', Some('=')) => (TokenKind::LessEqual, 2),
            ('>', Some('=')) => (TokenKind::GreaterEqual, 2),
            ('=', _) => (TokenKind::Equal, 1),
            ('<', _) => (TokenKind::Less, 1),
            ('>', _) => (TokenKind::Greater, 1),
            ('(', _) => (TokenKind::LParen, 1),
            (')', _) => (TokenKind::RParen, 1),
            ('{', _) => (TokenKind::LBrace, 1),
            ('}', _) => (TokenKind::RBrace, 1),
            (';', _) => (TokenKind::Semicolon, 1),
            (',', _) => (TokenKind::Comma, 1),
            ('.', _) => (TokenKind::Dot, 1),
            ('+', _) => (TokenKind::Plus, 1),
            ('-', _) => (TokenKind::Minus, 1),
            ('*', _) => (TokenKind::Star, 1),
            ('/', _) => (TokenKind::Slash, 1),
            _ => return Err(format!("unexpected character '{c}' at offset {i}")),
        };
        out.push(Token { kind, offset: i });
        i += width;
    }
    out.push(Token {
        kind: TokenKind::Eof,
        offset: chars.len(),
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recognizes_control_flow_keywords() {
        let tokens = lex("if else while break continue return").unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.kind.clone()).collect::<Vec<_>>(),
            vec![
                TokenKind::If,
                TokenKind::Else,
                TokenKind::While,
                TokenKind::Break,
                TokenKind::Continue,
                TokenKind::Return,
                TokenKind::Eof
            ]
        );
    }
    #[test]
    fn reports_unknown_character() {
        assert!(lex("fn main() { @ }").unwrap_err().contains("offset"));
    }
}
