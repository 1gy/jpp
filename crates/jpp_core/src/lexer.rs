//! Lexer for JSONPath queries

use std::iter::Peekable;
use std::str::Chars;

/// Token types for JSONPath
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// Root identifier `$`
    Root,
    /// Current node `@`
    At,
    /// Single dot `.`
    Dot,
    /// Double dot `..`
    DotDot,
    /// Opening bracket `[`
    BracketOpen,
    /// Closing bracket `]`
    BracketClose,
    /// Opening parenthesis `(`
    ParenOpen,
    /// Closing parenthesis `)`
    ParenClose,
    /// Wildcard `*`
    Wildcard,
    /// Colon `:`
    Colon,
    /// Comma `,`
    Comma,
    /// Question mark `?` (filter indicator)
    Question,
    /// Less than `<`
    LessThan,
    /// Greater than `>`
    GreaterThan,
    /// Less than or equal `<=`
    LessEq,
    /// Greater than or equal `>=`
    GreaterEq,
    /// Equal `==`
    Equal,
    /// Not equal `!=`
    NotEqual,
    /// Logical AND `&&`
    And,
    /// Logical OR `||`
    Or,
    /// Logical NOT `!`
    Not,
    /// Boolean true literal
    True,
    /// Boolean false literal
    False,
    /// Null literal
    Null,
    /// Identifier (unquoted key name)
    Ident(String),
    /// String literal (single or double quoted)
    String(String),
    /// Number (integer or floating-point)
    Number(f64),
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
            '@' => {
                self.advance();
                TokenKind::At
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
            '(' => {
                self.advance();
                TokenKind::ParenOpen
            }
            ')' => {
                self.advance();
                TokenKind::ParenClose
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
            '?' => {
                self.advance();
                TokenKind::Question
            }
            '<' => {
                self.advance();
                if self.chars.peek() == Some(&'=') {
                    self.advance();
                    TokenKind::LessEq
                } else {
                    TokenKind::LessThan
                }
            }
            '>' => {
                self.advance();
                if self.chars.peek() == Some(&'=') {
                    self.advance();
                    TokenKind::GreaterEq
                } else {
                    TokenKind::GreaterThan
                }
            }
            '=' => {
                self.advance();
                if self.chars.peek() == Some(&'=') {
                    self.advance();
                    TokenKind::Equal
                } else {
                    return Err(LexerError {
                        message: "expected '==' but found single '='".to_string(),
                        position: start_pos,
                    });
                }
            }
            '!' => {
                self.advance();
                if self.chars.peek() == Some(&'=') {
                    self.advance();
                    TokenKind::NotEqual
                } else {
                    TokenKind::Not
                }
            }
            '&' => {
                self.advance();
                if self.chars.peek() == Some(&'&') {
                    self.advance();
                    TokenKind::And
                } else {
                    return Err(LexerError {
                        message: "expected '&&' but found single '&'".to_string(),
                        position: start_pos,
                    });
                }
            }
            '|' => {
                self.advance();
                if self.chars.peek() == Some(&'|') {
                    self.advance();
                    TokenKind::Or
                } else {
                    return Err(LexerError {
                        message: "expected '||' but found single '|'".to_string(),
                        position: start_pos,
                    });
                }
            }
            '\'' | '"' => self.read_string()?,
            '-' | '0'..='9' => self.read_number()?,
            _ if is_ident_start(ch) => self.read_ident_or_keyword(),
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

    /// Read 4 hex digits for \uXXXX escape and return the code point
    fn read_unicode_escape(&mut self) -> Result<u32, LexerError> {
        let mut hex = String::with_capacity(4);
        for _ in 0..4 {
            match self.advance() {
                Some(ch) if ch.is_ascii_hexdigit() => hex.push(ch),
                _ => {
                    return Err(LexerError {
                        message: "invalid unicode escape: expected 4 hex digits".to_string(),
                        position: self.position,
                    });
                }
            }
        }
        u32::from_str_radix(&hex, 16).map_err(|_| LexerError {
            message: "invalid unicode escape".to_string(),
            position: self.position,
        })
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
                        'b' => value.push('\x08'),
                        'f' => value.push('\x0C'),
                        '/' => value.push('/'),
                        'u' => {
                            let code = self.read_unicode_escape()?;
                            // Check for surrogate pair
                            if (0xD800..=0xDBFF).contains(&code) {
                                // High surrogate - expect \uXXXX low surrogate
                                if self.advance() != Some('\\') || self.advance() != Some('u') {
                                    return Err(LexerError {
                                        message: "invalid surrogate pair".to_string(),
                                        position: self.position,
                                    });
                                }
                                let low = self.read_unicode_escape()?;
                                if !(0xDC00..=0xDFFF).contains(&low) {
                                    return Err(LexerError {
                                        message: "invalid low surrogate".to_string(),
                                        position: self.position,
                                    });
                                }
                                // Combine surrogate pair
                                let combined = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                                if let Some(ch) = char::from_u32(combined) {
                                    value.push(ch);
                                } else {
                                    return Err(LexerError {
                                        message: "invalid unicode code point".to_string(),
                                        position: self.position,
                                    });
                                }
                            } else if let Some(ch) = char::from_u32(code) {
                                value.push(ch);
                            } else {
                                return Err(LexerError {
                                    message: "invalid unicode code point".to_string(),
                                    position: self.position,
                                });
                            }
                        }
                        _ => {
                            return Err(LexerError {
                                message: format!("invalid escape sequence: \\{escaped}"),
                                position: self.position - 1,
                            });
                        }
                    }
                }
                Some(ch) => {
                    // RFC 9535: Control characters (U+0000 to U+001F) must be escaped
                    if (ch as u32) <= 0x1F {
                        return Err(LexerError {
                            message: format!("unescaped control character U+{:04X}", ch as u32),
                            position: self.position - 1,
                        });
                    }
                    value.push(ch)
                }
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

        // Optional leading minus sign
        if self.chars.peek() == Some(&'-')
            && let Some(ch) = self.advance()
        {
            num_str.push(ch);
        }

        // Integer part
        let int_start = num_str.len();
        while let Some(&ch) = self.chars.peek() {
            if ch.is_ascii_digit() {
                if let Some(digit) = self.advance() {
                    num_str.push(digit);
                }
            } else {
                break;
            }
        }
        let int_part = num_str[int_start..].to_string();

        // RFC 9535: Reject leading zeros (e.g., "01", "007") but allow "0"
        if int_part.len() > 1 && int_part.starts_with('0') {
            return Err(LexerError {
                message: "leading zeros not allowed".to_string(),
                position: start_pos,
            });
        }

        let is_negative = num_str.starts_with('-');
        let mut has_fraction_or_exp = false;

        // Decimal part (optional)
        if self.chars.peek() == Some(&'.') {
            // Peek ahead to ensure it's followed by a digit (not another dot like ..)
            let mut chars_clone = self.chars.clone();
            chars_clone.next(); // consume the '.'
            if chars_clone.peek().is_some_and(|c| c.is_ascii_digit()) {
                has_fraction_or_exp = true;
                if let Some(dot) = self.advance() {
                    num_str.push(dot); // consume '.'
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
            }
        }

        // Exponent part (optional)
        if self.chars.peek().is_some_and(|&c| c == 'e' || c == 'E') {
            has_fraction_or_exp = true;
            if let Some(e) = self.advance() {
                num_str.push(e); // consume 'e' or 'E'
            }

            // Optional sign for exponent
            if self.chars.peek().is_some_and(|&c| c == '+' || c == '-')
                && let Some(sign) = self.advance()
            {
                num_str.push(sign);
            }

            // Exponent digits (required)
            let exp_start = num_str.len();
            while let Some(&ch) = self.chars.peek() {
                if ch.is_ascii_digit() {
                    if let Some(digit) = self.advance() {
                        num_str.push(digit);
                    }
                } else {
                    break;
                }
            }
            if num_str.len() == exp_start || num_str.ends_with('+') || num_str.ends_with('-') {
                return Err(LexerError {
                    message: "invalid exponent in number".to_string(),
                    position: start_pos,
                });
            }
        }

        if num_str.is_empty() || num_str == "-" {
            return Err(LexerError {
                message: "invalid number".to_string(),
                position: start_pos,
            });
        }

        // RFC 9535: Reject "-0" as integer (but allow -0.5, -0e1, etc.)
        if is_negative && int_part == "0" && !has_fraction_or_exp {
            return Err(LexerError {
                message: "-0 is not allowed".to_string(),
                position: start_pos,
            });
        }

        let value: f64 = num_str.parse().map_err(|_| LexerError {
            message: "number out of range".to_string(),
            position: start_pos,
        })?;

        Ok(TokenKind::Number(value))
    }

    fn read_ident_or_keyword(&mut self) -> TokenKind {
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

        // Check for keywords
        match ident.as_str() {
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            _ => TokenKind::Ident(ident),
        }
    }
}

/// Check if character is valid as the start of an identifier (RFC 9535 name-first)
/// name-first = ALPHA / "_" / %x80-D7FF / %xE000-10FFFF
fn is_ident_start(ch: char) -> bool {
    let code = ch as u32;
    ch.is_ascii_alphabetic()
        || ch == '_'
        || (0x80..=0xD7FF).contains(&code)
        || (0xE000..=0x10FFFF).contains(&code)
}

/// Check if character is valid within an identifier (RFC 9535 name-char)
/// name-char = name-first / DIGIT
fn is_ident_char(ch: char) -> bool {
    is_ident_start(ch) || ch.is_ascii_digit()
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
                &TokenKind::Number(0.0),
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
                &TokenKind::Number(-1.0),
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

    #[test]
    fn test_current_node() {
        let tokens = Lexer::new("@.price").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::At,
                &TokenKind::Dot,
                &TokenKind::Ident("price".to_string())
            ]
        );
    }

    #[test]
    fn test_filter_indicator() {
        let tokens = Lexer::new("$[?@.price]").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::Root,
                &TokenKind::BracketOpen,
                &TokenKind::Question,
                &TokenKind::At,
                &TokenKind::Dot,
                &TokenKind::Ident("price".to_string()),
                &TokenKind::BracketClose
            ]
        );
    }

    #[test]
    fn test_comparison_operators() {
        let tokens = Lexer::new("< > <= >= == !=").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::LessThan,
                &TokenKind::GreaterThan,
                &TokenKind::LessEq,
                &TokenKind::GreaterEq,
                &TokenKind::Equal,
                &TokenKind::NotEqual
            ]
        );
    }

    #[test]
    fn test_logical_operators() {
        let tokens = Lexer::new("&& || !").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![&TokenKind::And, &TokenKind::Or, &TokenKind::Not]
        );
    }

    #[test]
    fn test_parentheses() {
        let tokens = Lexer::new("(@.a && @.b)").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::ParenOpen,
                &TokenKind::At,
                &TokenKind::Dot,
                &TokenKind::Ident("a".to_string()),
                &TokenKind::And,
                &TokenKind::At,
                &TokenKind::Dot,
                &TokenKind::Ident("b".to_string()),
                &TokenKind::ParenClose
            ]
        );
    }

    #[test]
    fn test_keywords() {
        let tokens = Lexer::new("true false null").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![&TokenKind::True, &TokenKind::False, &TokenKind::Null]
        );
    }

    #[test]
    fn test_filter_expression() {
        let tokens = Lexer::new("$[?@.price < 10]").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::Root,
                &TokenKind::BracketOpen,
                &TokenKind::Question,
                &TokenKind::At,
                &TokenKind::Dot,
                &TokenKind::Ident("price".to_string()),
                &TokenKind::LessThan,
                &TokenKind::Number(10.0),
                &TokenKind::BracketClose
            ]
        );
    }

    #[test]
    fn test_complex_filter() {
        let tokens = Lexer::new("$[?@.price >= 10 && @.available == true]")
            .tokenize()
            .unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::Root,
                &TokenKind::BracketOpen,
                &TokenKind::Question,
                &TokenKind::At,
                &TokenKind::Dot,
                &TokenKind::Ident("price".to_string()),
                &TokenKind::GreaterEq,
                &TokenKind::Number(10.0),
                &TokenKind::And,
                &TokenKind::At,
                &TokenKind::Dot,
                &TokenKind::Ident("available".to_string()),
                &TokenKind::Equal,
                &TokenKind::True,
                &TokenKind::BracketClose
            ]
        );
    }

    #[test]
    fn test_invalid_single_ampersand() {
        let result = Lexer::new("&").tokenize();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("expected '&&'"));
    }

    #[test]
    fn test_invalid_single_pipe() {
        let result = Lexer::new("|").tokenize();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("expected '||'"));
    }

    #[test]
    fn test_invalid_single_equals() {
        let result = Lexer::new("=").tokenize();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("expected '=='"));
    }

    // ========== Floating-Point Number Tests ==========

    #[test]
    fn test_float_decimal() {
        let tokens = Lexer::new("1.5").tokenize().unwrap();
        assert_eq!(kinds(&tokens), vec![&TokenKind::Number(1.5)]);
    }

    #[test]
    fn test_float_multiple_decimals() {
        let tokens = Lexer::new("3.12345").tokenize().unwrap();
        assert_eq!(kinds(&tokens), vec![&TokenKind::Number(3.12345)]);
    }

    #[test]
    fn test_float_exponent() {
        let tokens = Lexer::new("1e10").tokenize().unwrap();
        assert_eq!(kinds(&tokens), vec![&TokenKind::Number(1e10)]);
    }

    #[test]
    fn test_float_exponent_uppercase() {
        let tokens = Lexer::new("1E10").tokenize().unwrap();
        assert_eq!(kinds(&tokens), vec![&TokenKind::Number(1e10)]);
    }

    #[test]
    fn test_float_exponent_negative() {
        let tokens = Lexer::new("1e-3").tokenize().unwrap();
        assert_eq!(kinds(&tokens), vec![&TokenKind::Number(1e-3)]);
    }

    #[test]
    fn test_float_exponent_positive() {
        let tokens = Lexer::new("1e+3").tokenize().unwrap();
        assert_eq!(kinds(&tokens), vec![&TokenKind::Number(1e3)]);
    }

    #[test]
    fn test_float_full() {
        let tokens = Lexer::new("1.5e-3").tokenize().unwrap();
        assert_eq!(kinds(&tokens), vec![&TokenKind::Number(1.5e-3)]);
    }

    #[test]
    fn test_negative_float() {
        let tokens = Lexer::new("-1.5").tokenize().unwrap();
        assert_eq!(kinds(&tokens), vec![&TokenKind::Number(-1.5)]);
    }

    // ========== Unicode Identifier Tests ==========

    #[test]
    fn test_unicode_emoji_identifier() {
        let tokens = Lexer::new("$.☺").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::Root,
                &TokenKind::Dot,
                &TokenKind::Ident("☺".to_string())
            ]
        );
    }

    #[test]
    fn test_unicode_japanese_identifier() {
        let tokens = Lexer::new("$.日本語").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::Root,
                &TokenKind::Dot,
                &TokenKind::Ident("日本語".to_string())
            ]
        );
    }

    #[test]
    fn test_unicode_accented_identifier() {
        let tokens = Lexer::new("$.émoji").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::Root,
                &TokenKind::Dot,
                &TokenKind::Ident("émoji".to_string())
            ]
        );
    }

    #[test]
    fn test_unicode_mixed_identifier() {
        let tokens = Lexer::new("$.hello世界123").tokenize().unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                &TokenKind::Root,
                &TokenKind::Dot,
                &TokenKind::Ident("hello世界123".to_string())
            ]
        );
    }
}
