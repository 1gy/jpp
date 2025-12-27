//! Lexer for JSONPath queries

use std::iter::Peekable;
use std::str::Chars;

/// Token types for JSONPath
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// Root identifier `$`
    Root,
    /// Single dot `.`
    Dot,
    /// Double dot `..`
    DotDot,
    /// Opening bracket `[`
    BracketOpen,
    /// Closing bracket `]`
    BracketClose,
    /// Wildcard `*`
    Wildcard,
    /// Colon `:`
    Colon,
    /// Comma `,`
    Comma,
    /// Identifier (unquoted key name)
    Ident(String),
    /// String literal (single or double quoted)
    String(String),
    /// Integer number
    Number(i64),
}

/// Token with position information
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub position: usize,
}

/// Lexer error
#[derive(Debug, Clone, PartialEq)]
pub struct LexerError {
    pub message: String,
    pub position: usize,
}

impl std::fmt::Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "at position {}: {}", self.position, self.message)
    }
}

/// Lexer for tokenizing JSONPath queries
pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    position: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
            position: 0,
        }
    }

    /// Tokenize the entire input
    pub fn tokenize(mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();

        while let Some(token) = self.next_token()? {
            tokens.push(token);
        }

        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Option<Token>, LexerError> {
        self.skip_whitespace();

        let Some(&ch) = self.chars.peek() else {
            return Ok(None);
        };

        let start_pos = self.position;

        let kind = match ch {
            '$' => {
                self.advance();
                TokenKind::Root
            }
            '.' => {
                self.advance();
                if self.chars.peek() == Some(&'.') {
                    self.advance();
                    TokenKind::DotDot
                } else {
                    TokenKind::Dot
                }
            }
            '[' => {
                self.advance();
                TokenKind::BracketOpen
            }
            ']' => {
                self.advance();
                TokenKind::BracketClose
            }
            '*' => {
                self.advance();
                TokenKind::Wildcard
            }
            ':' => {
                self.advance();
                TokenKind::Colon
            }
            ',' => {
                self.advance();
                TokenKind::Comma
            }
            '\'' | '"' => self.read_string()?,
            '-' | '0'..='9' => self.read_number()?,
            _ if is_ident_start(ch) => self.read_ident(),
            _ => {
                return Err(LexerError {
                    message: format!("unexpected character: '{ch}'"),
                    position: self.position,
                });
            }
        };

        Ok(Some(Token {
            kind,
            position: start_pos,
        }))
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.next();
        if ch.is_some() {
            self.position += 1;
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(&ch) = self.chars.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_string(&mut self) -> Result<TokenKind, LexerError> {
        let quote = self.advance().ok_or_else(|| LexerError {
            message: "unexpected end of input".to_string(),
            position: self.position,
        })?;

        let mut value = String::new();
        let start_pos = self.position;

        loop {
            match self.advance() {
                Some(ch) if ch == quote => break,
                Some('\\') => {
                    let escaped = self.advance().ok_or_else(|| LexerError {
                        message: "unexpected end of input in escape sequence".to_string(),
                        position: self.position,
                    })?;
                    match escaped {
                        'n' => value.push('\n'),
                        't' => value.push('\t'),
                        'r' => value.push('\r'),
                        '\\' => value.push('\\'),
                        '\'' => value.push('\''),
                        '"' => value.push('"'),
                        _ => {
                            return Err(LexerError {
                                message: format!("invalid escape sequence: \\{escaped}"),
                                position: self.position - 1,
                            });
                        }
                    }
                }
                Some(ch) => value.push(ch),
                None => {
                    return Err(LexerError {
                        message: "unterminated string".to_string(),
                        position: start_pos,
                    });
                }
            }
        }

        Ok(TokenKind::String(value))
    }

    fn read_number(&mut self) -> Result<TokenKind, LexerError> {
        let start_pos = self.position;
        let mut num_str = String::new();

        if self.chars.peek() == Some(&'-')
            && let Some(ch) = self.advance()
        {
            num_str.push(ch);
        }

        while let Some(&ch) = self.chars.peek() {
            if ch.is_ascii_digit() {
                if let Some(digit) = self.advance() {
                    num_str.push(digit);
                }
            } else {
                break;
            }
        }

        if num_str.is_empty() || num_str == "-" {
            return Err(LexerError {
                message: "invalid number".to_string(),
                position: start_pos,
            });
        }

        let value: i64 = num_str.parse().map_err(|_| LexerError {
            message: "number out of range".to_string(),
            position: start_pos,
        })?;

        Ok(TokenKind::Number(value))
    }

    fn read_ident(&mut self) -> TokenKind {
        let mut ident = String::new();

        while let Some(&ch) = self.chars.peek() {
            if is_ident_char(ch) {
                if let Some(c) = self.advance() {
                    ident.push(c);
                }
            } else {
                break;
            }
        }

        TokenKind::Ident(ident)
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

fn is_ident_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn kinds(tokens: &[Token]) -> Vec<&TokenKind> {
        tokens.iter().map(|t| &t.kind).collect()
    }

    #[test]
    fn test_basic_tokens() {
        let tokens = Lexer::new("$.foo").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::Root,
                &TokenKind::Dot,
                &TokenKind::Ident("foo".to_string())
            ]
        );
    }

    #[test]
    fn test_bracket_notation() {
        let tokens = Lexer::new("$['foo']").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::Root,
                &TokenKind::BracketOpen,
                &TokenKind::String("foo".to_string()),
                &TokenKind::BracketClose
            ]
        );
    }

    #[test]
    fn test_array_index() {
        let tokens = Lexer::new("$[0]").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::Root,
                &TokenKind::BracketOpen,
                &TokenKind::Number(0),
                &TokenKind::BracketClose
            ]
        );
    }

    #[test]
    fn test_negative_index() {
        let tokens = Lexer::new("$[-1]").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::Root,
                &TokenKind::BracketOpen,
                &TokenKind::Number(-1),
                &TokenKind::BracketClose
            ]
        );
    }

    #[test]
    fn test_wildcard() {
        let tokens = Lexer::new("$[*]").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::Root,
                &TokenKind::BracketOpen,
                &TokenKind::Wildcard,
                &TokenKind::BracketClose
            ]
        );
    }

    #[test]
    fn test_descendant() {
        let tokens = Lexer::new("$..foo").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::Root,
                &TokenKind::DotDot,
                &TokenKind::Ident("foo".to_string())
            ]
        );
    }

    #[test]
    fn test_token_positions() {
        let tokens = Lexer::new("$.foo").tokenize().unwrap();
        assert_eq!(tokens[0].position, 0); // $
        assert_eq!(tokens[1].position, 1); // .
        assert_eq!(tokens[2].position, 2); // foo
    }
}
