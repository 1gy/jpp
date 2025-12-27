//! Parser for JSONPath queries

use crate::ast::{JsonPath, Segment, Selector};
use crate::lexer::{Lexer, LexerError, Token, TokenKind};

/// Parser error
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub position: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "at position {}, {}", self.position, self.message)
    }
}

impl From<LexerError> for ParseError {
    fn from(e: LexerError) -> Self {
        Self {
            message: e.message,
            position: e.position,
        }
    }
}

/// Parser for JSONPath queries
pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, index: 0 }
    }

    /// Parse a JSONPath query string
    pub fn parse(input: &str) -> Result<JsonPath, ParseError> {
        let tokens = Lexer::new(input).tokenize()?;
        let mut parser = Self::new(tokens);
        parser.parse_jsonpath()
    }

    fn parse_jsonpath(&mut self) -> Result<JsonPath, ParseError> {
        // Expect root identifier
        if self.current_kind() != Some(&TokenKind::Root) {
            return Err(ParseError {
                message: "JSONPath must start with '$'".to_string(),
                position: 0,
            });
        }
        self.advance();

        let mut segments = Vec::new();

        while self.current().is_some() {
            let segment = self.parse_segment()?;
            segments.push(segment);
        }

        Ok(JsonPath::new(segments))
    }

    fn parse_segment(&mut self) -> Result<Segment, ParseError> {
        match self.current_kind() {
            Some(TokenKind::DotDot) => {
                self.advance();
                let selectors = self.parse_selectors_after_dot()?;
                Ok(Segment::Descendant(selectors))
            }
            Some(TokenKind::Dot) => {
                self.advance();
                let selectors = self.parse_selectors_after_dot()?;
                Ok(Segment::Child(selectors))
            }
            Some(TokenKind::BracketOpen) => {
                let selectors = self.parse_bracket_selectors()?;
                Ok(Segment::Child(selectors))
            }
            Some(kind) => Err(ParseError {
                message: format!("unexpected token: {kind:?}"),
                position: self.current_position(),
            }),
            None => Err(ParseError {
                message: "unexpected end of input".to_string(),
                position: self.current_position(),
            }),
        }
    }

    fn parse_selectors_after_dot(&mut self) -> Result<Vec<Selector>, ParseError> {
        match self.current_kind().cloned() {
            Some(TokenKind::Ident(name)) => {
                self.advance();
                Ok(vec![Selector::Name(name)])
            }
            Some(TokenKind::Wildcard) => {
                self.advance();
                Ok(vec![Selector::Wildcard])
            }
            Some(TokenKind::BracketOpen) => self.parse_bracket_selectors(),
            Some(kind) => Err(ParseError {
                message: format!("expected identifier or wildcard after '.', got {kind:?}"),
                position: self.current_position(),
            }),
            None => Err(ParseError {
                message: "expected identifier or wildcard after '.'".to_string(),
                position: self.current_position(),
            }),
        }
    }

    fn parse_bracket_selectors(&mut self) -> Result<Vec<Selector>, ParseError> {
        // Consume '['
        if self.current_kind() != Some(&TokenKind::BracketOpen) {
            return Err(ParseError {
                message: "expected '['".to_string(),
                position: self.current_position(),
            });
        }
        self.advance();

        let mut selectors = Vec::new();

        loop {
            let selector = self.parse_selector()?;
            selectors.push(selector);

            match self.current_kind() {
                Some(TokenKind::Comma) => {
                    self.advance();
                    continue;
                }
                Some(TokenKind::BracketClose) => {
                    self.advance();
                    break;
                }
                Some(kind) => {
                    return Err(ParseError {
                        message: format!("expected ',' or ']', got {kind:?}"),
                        position: self.current_position(),
                    });
                }
                None => {
                    return Err(ParseError {
                        message: "unclosed bracket".to_string(),
                        position: self.current_position(),
                    });
                }
            }
        }

        Ok(selectors)
    }

    fn parse_selector(&mut self) -> Result<Selector, ParseError> {
        match self.current_kind().cloned() {
            Some(TokenKind::Wildcard) => {
                self.advance();
                Ok(Selector::Wildcard)
            }
            Some(TokenKind::String(s)) => {
                self.advance();
                Ok(Selector::Name(s))
            }
            Some(TokenKind::Number(_)) | Some(TokenKind::Colon) => self.parse_index_or_slice(),
            Some(kind) => Err(ParseError {
                message: format!("unexpected token in selector: {kind:?}"),
                position: self.current_position(),
            }),
            None => Err(ParseError {
                message: "unexpected end of input in selector".to_string(),
                position: self.current_position(),
            }),
        }
    }

    fn parse_index_or_slice(&mut self) -> Result<Selector, ParseError> {
        // Parse: number, :, number:, :number, number:number, number:number:number, etc.
        let start = self.try_parse_number();

        if self.current_kind() != Some(&TokenKind::Colon) {
            // Just an index
            return match start {
                Some(n) => Ok(Selector::Index(n)),
                None => Err(ParseError {
                    message: "expected number".to_string(),
                    position: self.current_position(),
                }),
            };
        }

        // It's a slice
        self.advance(); // consume first ':'

        let end = self.try_parse_number();

        let step = if self.current_kind() == Some(&TokenKind::Colon) {
            self.advance(); // consume second ':'
            self.try_parse_number()
        } else {
            None
        };

        Ok(Selector::Slice { start, end, step })
    }

    fn try_parse_number(&mut self) -> Option<i64> {
        if let Some(TokenKind::Number(n)) = self.current_kind() {
            let n = *n;
            self.advance();
            Some(n)
        } else {
            None
        }
    }

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn current_kind(&self) -> Option<&TokenKind> {
        self.current().map(|t| &t.kind)
    }

    fn current_position(&self) -> usize {
        self.current().map(|t| t.position).unwrap_or(
            // If past the end, use position after last token
            self.tokens.last().map(|t| t.position + 1).unwrap_or(0),
        )
    }

    fn advance(&mut self) {
        self.index += 1;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_root_only() {
        let path = Parser::parse("$").unwrap();
        assert_eq!(path.segments.len(), 0);
    }

    #[test]
    fn test_parse_simple_name() {
        let path = Parser::parse("$.foo").unwrap();
        assert_eq!(path.segments.len(), 1);
        assert_eq!(
            path.segments[0],
            Segment::Child(vec![Selector::Name("foo".to_string())])
        );
    }

    #[test]
    fn test_parse_bracket_name() {
        let path = Parser::parse("$['foo']").unwrap();
        assert_eq!(path.segments.len(), 1);
        assert_eq!(
            path.segments[0],
            Segment::Child(vec![Selector::Name("foo".to_string())])
        );
    }

    #[test]
    fn test_parse_index() {
        let path = Parser::parse("$[0]").unwrap();
        assert_eq!(path.segments.len(), 1);
        assert_eq!(path.segments[0], Segment::Child(vec![Selector::Index(0)]));
    }

    #[test]
    fn test_parse_negative_index() {
        let path = Parser::parse("$[-1]").unwrap();
        assert_eq!(path.segments.len(), 1);
        assert_eq!(path.segments[0], Segment::Child(vec![Selector::Index(-1)]));
    }

    #[test]
    fn test_parse_wildcard() {
        let path = Parser::parse("$[*]").unwrap();
        assert_eq!(path.segments.len(), 1);
        assert_eq!(path.segments[0], Segment::Child(vec![Selector::Wildcard]));
    }

    #[test]
    fn test_parse_dot_wildcard() {
        let path = Parser::parse("$.*").unwrap();
        assert_eq!(path.segments.len(), 1);
        assert_eq!(path.segments[0], Segment::Child(vec![Selector::Wildcard]));
    }

    #[test]
    fn test_parse_descendant() {
        let path = Parser::parse("$..foo").unwrap();
        assert_eq!(path.segments.len(), 1);
        assert_eq!(
            path.segments[0],
            Segment::Descendant(vec![Selector::Name("foo".to_string())])
        );
    }

    #[test]
    fn test_parse_slice() {
        let path = Parser::parse("$[1:3]").unwrap();
        assert_eq!(path.segments.len(), 1);
        assert_eq!(
            path.segments[0],
            Segment::Child(vec![Selector::Slice {
                start: Some(1),
                end: Some(3),
                step: None
            }])
        );
    }

    #[test]
    fn test_parse_complex_path() {
        let path = Parser::parse("$.store.book[0].author").unwrap();
        assert_eq!(path.segments.len(), 4);
    }
}
