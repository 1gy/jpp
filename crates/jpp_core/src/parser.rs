//! Parser for JSONPath queries

use crate::ast::{CompOp, Expr, JsonPath, Literal, LogicalOp, Segment, Selector};
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
            Some(TokenKind::Question) => {
                self.advance(); // consume '?'
                let expr = self.parse_expression()?;
                Ok(Selector::Filter(Box::new(expr)))
            }
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

    // ========== Expression Parsing ==========

    /// Parse an expression (entry point) - handles logical OR (lowest precedence)
    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_or_expression()
    }

    /// Parse logical OR expression: expr || expr
    fn parse_or_expression(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and_expression()?;

        while self.current_kind() == Some(&TokenKind::Or) {
            self.advance(); // consume '||'
            let right = self.parse_and_expression()?;
            left = Expr::Logical {
                left: Box::new(left),
                op: LogicalOp::Or,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse logical AND expression: expr && expr
    fn parse_and_expression(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison_expression()?;

        while self.current_kind() == Some(&TokenKind::And) {
            self.advance(); // consume '&&'
            let right = self.parse_comparison_expression()?;
            left = Expr::Logical {
                left: Box::new(left),
                op: LogicalOp::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse comparison expression: expr op expr
    fn parse_comparison_expression(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_unary_expression()?;

        let op = match self.current_kind() {
            Some(TokenKind::Equal) => Some(CompOp::Eq),
            Some(TokenKind::NotEqual) => Some(CompOp::Ne),
            Some(TokenKind::LessThan) => Some(CompOp::Lt),
            Some(TokenKind::GreaterThan) => Some(CompOp::Gt),
            Some(TokenKind::LessEq) => Some(CompOp::Le),
            Some(TokenKind::GreaterEq) => Some(CompOp::Ge),
            _ => None,
        };

        if let Some(op) = op {
            self.advance(); // consume operator
            let right = self.parse_unary_expression()?;
            Ok(Expr::Comparison {
                left: Box::new(left),
                op,
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    /// Parse unary expression: !expr or atom
    fn parse_unary_expression(&mut self) -> Result<Expr, ParseError> {
        if self.current_kind() == Some(&TokenKind::Not) {
            self.advance(); // consume '!'
            let expr = self.parse_unary_expression()?;
            Ok(Expr::Not(Box::new(expr)))
        } else {
            self.parse_atom()
        }
    }

    /// Parse atom: @, $, literal, function call, or parenthesized expression
    fn parse_atom(&mut self) -> Result<Expr, ParseError> {
        match self.current_kind().cloned() {
            Some(TokenKind::At) => {
                self.advance(); // consume '@'
                self.parse_path_or_node(Expr::CurrentNode)
            }
            Some(TokenKind::Root) => {
                self.advance(); // consume '$'
                self.parse_path_or_node(Expr::RootNode)
            }
            Some(TokenKind::True) => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(true)))
            }
            Some(TokenKind::False) => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(false)))
            }
            Some(TokenKind::Null) => {
                self.advance();
                Ok(Expr::Literal(Literal::Null))
            }
            Some(TokenKind::Number(n)) => {
                self.advance();
                Ok(Expr::Literal(Literal::Number(n)))
            }
            Some(TokenKind::String(s)) => {
                self.advance();
                Ok(Expr::Literal(Literal::String(s)))
            }
            Some(TokenKind::Ident(name)) => {
                self.advance();
                // Check if this is a function call
                if self.current_kind() == Some(&TokenKind::ParenOpen) {
                    self.parse_function_call(name)
                } else {
                    Err(ParseError {
                        message: format!("unexpected identifier '{name}' in expression"),
                        position: self.current_position(),
                    })
                }
            }
            Some(TokenKind::ParenOpen) => {
                self.advance(); // consume '('
                let expr = self.parse_expression()?;
                if self.current_kind() != Some(&TokenKind::ParenClose) {
                    return Err(ParseError {
                        message: "expected ')' after expression".to_string(),
                        position: self.current_position(),
                    });
                }
                self.advance(); // consume ')'
                Ok(expr)
            }
            Some(kind) => Err(ParseError {
                message: format!("unexpected token in expression: {kind:?}"),
                position: self.current_position(),
            }),
            None => Err(ParseError {
                message: "unexpected end of input in expression".to_string(),
                position: self.current_position(),
            }),
        }
    }

    /// Parse path segments after @ or $, or return the node itself
    fn parse_path_or_node(&mut self, start: Expr) -> Result<Expr, ParseError> {
        // Check if followed by path segments
        if !matches!(
            self.current_kind(),
            Some(TokenKind::Dot) | Some(TokenKind::DotDot) | Some(TokenKind::BracketOpen)
        ) {
            return Ok(start);
        }

        let mut segments = Vec::new();

        while matches!(
            self.current_kind(),
            Some(TokenKind::Dot) | Some(TokenKind::DotDot) | Some(TokenKind::BracketOpen)
        ) {
            let segment = self.parse_filter_path_segment()?;
            segments.push(segment);
        }

        Ok(Expr::Path {
            start: Box::new(start),
            segments,
        })
    }

    /// Parse a path segment within a filter expression (simpler than full segment parsing)
    fn parse_filter_path_segment(&mut self) -> Result<Segment, ParseError> {
        match self.current_kind() {
            Some(TokenKind::DotDot) => {
                self.advance();
                let selectors = self.parse_filter_selectors_after_dot()?;
                Ok(Segment::Descendant(selectors))
            }
            Some(TokenKind::Dot) => {
                self.advance();
                let selectors = self.parse_filter_selectors_after_dot()?;
                Ok(Segment::Child(selectors))
            }
            Some(TokenKind::BracketOpen) => {
                self.advance(); // consume '['
                let selector = self.parse_filter_bracket_selector()?;
                if self.current_kind() != Some(&TokenKind::BracketClose) {
                    return Err(ParseError {
                        message: "expected ']'".to_string(),
                        position: self.current_position(),
                    });
                }
                self.advance(); // consume ']'
                Ok(Segment::Child(vec![selector]))
            }
            _ => Err(ParseError {
                message: "expected path segment".to_string(),
                position: self.current_position(),
            }),
        }
    }

    /// Parse selectors after '.' or '..' in filter path
    fn parse_filter_selectors_after_dot(&mut self) -> Result<Vec<Selector>, ParseError> {
        match self.current_kind().cloned() {
            Some(TokenKind::Ident(name)) => {
                self.advance();
                Ok(vec![Selector::Name(name)])
            }
            Some(TokenKind::Wildcard) => {
                self.advance();
                Ok(vec![Selector::Wildcard])
            }
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

    /// Parse a bracket selector within filter path (name, index, wildcard, or slice)
    fn parse_filter_bracket_selector(&mut self) -> Result<Selector, ParseError> {
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
                message: format!("unexpected token in bracket selector: {kind:?}"),
                position: self.current_position(),
            }),
            None => Err(ParseError {
                message: "unexpected end of input in bracket selector".to_string(),
                position: self.current_position(),
            }),
        }
    }

    /// Parse a function call: name(args...)
    fn parse_function_call(&mut self, name: String) -> Result<Expr, ParseError> {
        // Consume '('
        if self.current_kind() != Some(&TokenKind::ParenOpen) {
            return Err(ParseError {
                message: "expected '(' after function name".to_string(),
                position: self.current_position(),
            });
        }
        self.advance();

        let mut args = Vec::new();

        // Check for empty argument list
        if self.current_kind() != Some(&TokenKind::ParenClose) {
            // Parse first argument
            args.push(self.parse_expression()?);

            // Parse remaining arguments
            while self.current_kind() == Some(&TokenKind::Comma) {
                self.advance(); // consume ','
                args.push(self.parse_expression()?);
            }
        }

        // Consume ')'
        if self.current_kind() != Some(&TokenKind::ParenClose) {
            return Err(ParseError {
                message: "expected ')' after function arguments".to_string(),
                position: self.current_position(),
            });
        }
        self.advance();

        Ok(Expr::FunctionCall { name, args })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
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

    // ========== Filter Expression Tests ==========

    #[test]
    fn test_parse_simple_filter() {
        let path = Parser::parse("$[?@.price]").unwrap();
        assert_eq!(path.segments.len(), 1);
        match &path.segments[0] {
            Segment::Child(selectors) => {
                assert_eq!(selectors.len(), 1);
                match &selectors[0] {
                    Selector::Filter(expr) => {
                        // Should be a path expression @.price
                        match expr.as_ref() {
                            Expr::Path { start, segments } => {
                                assert_eq!(**start, Expr::CurrentNode);
                                assert_eq!(segments.len(), 1);
                            }
                            _ => panic!("expected Path expression"),
                        }
                    }
                    _ => panic!("expected Filter selector"),
                }
            }
            _ => panic!("expected Child segment"),
        }
    }

    #[test]
    fn test_parse_filter_comparison() {
        let path = Parser::parse("$[?@.price < 10]").unwrap();
        assert_eq!(path.segments.len(), 1);
        match &path.segments[0] {
            Segment::Child(selectors) => {
                assert_eq!(selectors.len(), 1);
                match &selectors[0] {
                    Selector::Filter(expr) => match expr.as_ref() {
                        Expr::Comparison { left, op, right } => {
                            assert_eq!(*op, CompOp::Lt);
                            // left should be @.price
                            match left.as_ref() {
                                Expr::Path { start, .. } => {
                                    assert_eq!(**start, Expr::CurrentNode);
                                }
                                _ => panic!("expected Path on left"),
                            }
                            // right should be 10
                            assert_eq!(**right, Expr::Literal(Literal::Number(10)));
                        }
                        _ => panic!("expected Comparison expression"),
                    },
                    _ => panic!("expected Filter selector"),
                }
            }
            _ => panic!("expected Child segment"),
        }
    }

    #[test]
    fn test_parse_filter_logical_and() {
        let path = Parser::parse("$[?@.price < 10 && @.available]").unwrap();
        match &path.segments[0] {
            Segment::Child(selectors) => match &selectors[0] {
                Selector::Filter(expr) => match expr.as_ref() {
                    Expr::Logical { op, .. } => {
                        assert_eq!(*op, LogicalOp::And);
                    }
                    _ => panic!("expected Logical expression"),
                },
                _ => panic!("expected Filter selector"),
            },
            _ => panic!("expected Child segment"),
        }
    }

    #[test]
    fn test_parse_filter_logical_or() {
        let path = Parser::parse("$[?@.a || @.b]").unwrap();
        match &path.segments[0] {
            Segment::Child(selectors) => match &selectors[0] {
                Selector::Filter(expr) => match expr.as_ref() {
                    Expr::Logical { op, .. } => {
                        assert_eq!(*op, LogicalOp::Or);
                    }
                    _ => panic!("expected Logical expression"),
                },
                _ => panic!("expected Filter selector"),
            },
            _ => panic!("expected Child segment"),
        }
    }

    #[test]
    fn test_parse_filter_not() {
        let path = Parser::parse("$[?!@.archived]").unwrap();
        match &path.segments[0] {
            Segment::Child(selectors) => match &selectors[0] {
                Selector::Filter(expr) => match expr.as_ref() {
                    Expr::Not(inner) => {
                        // inner should be @.archived
                        match inner.as_ref() {
                            Expr::Path { start, .. } => {
                                assert_eq!(**start, Expr::CurrentNode);
                            }
                            _ => panic!("expected Path inside Not"),
                        }
                    }
                    _ => panic!("expected Not expression"),
                },
                _ => panic!("expected Filter selector"),
            },
            _ => panic!("expected Child segment"),
        }
    }

    #[test]
    fn test_parse_filter_function_call() {
        let path = Parser::parse("$[?length(@.items) > 0]").unwrap();
        match &path.segments[0] {
            Segment::Child(selectors) => match &selectors[0] {
                Selector::Filter(expr) => match expr.as_ref() {
                    Expr::Comparison { left, op, right } => {
                        assert_eq!(*op, CompOp::Gt);
                        // left should be function call
                        match left.as_ref() {
                            Expr::FunctionCall { name, args } => {
                                assert_eq!(name, "length");
                                assert_eq!(args.len(), 1);
                            }
                            _ => panic!("expected FunctionCall on left"),
                        }
                        assert_eq!(**right, Expr::Literal(Literal::Number(0)));
                    }
                    _ => panic!("expected Comparison expression"),
                },
                _ => panic!("expected Filter selector"),
            },
            _ => panic!("expected Child segment"),
        }
    }

    #[test]
    fn test_parse_filter_with_literals() {
        let path = Parser::parse("$[?@.name == \"test\"]").unwrap();
        match &path.segments[0] {
            Segment::Child(selectors) => match &selectors[0] {
                Selector::Filter(expr) => match expr.as_ref() {
                    Expr::Comparison { right, .. } => {
                        assert_eq!(**right, Expr::Literal(Literal::String("test".to_string())));
                    }
                    _ => panic!("expected Comparison expression"),
                },
                _ => panic!("expected Filter selector"),
            },
            _ => panic!("expected Child segment"),
        }
    }

    #[test]
    fn test_parse_filter_with_null() {
        let path = Parser::parse("$[?@.value != null]").unwrap();
        match &path.segments[0] {
            Segment::Child(selectors) => match &selectors[0] {
                Selector::Filter(expr) => match expr.as_ref() {
                    Expr::Comparison { op, right, .. } => {
                        assert_eq!(*op, CompOp::Ne);
                        assert_eq!(**right, Expr::Literal(Literal::Null));
                    }
                    _ => panic!("expected Comparison expression"),
                },
                _ => panic!("expected Filter selector"),
            },
            _ => panic!("expected Child segment"),
        }
    }

    #[test]
    fn test_parse_filter_parentheses() {
        let path = Parser::parse("$[?(@.a || @.b) && @.c]").unwrap();
        match &path.segments[0] {
            Segment::Child(selectors) => match &selectors[0] {
                Selector::Filter(expr) => match expr.as_ref() {
                    Expr::Logical { left, op, .. } => {
                        assert_eq!(*op, LogicalOp::And);
                        // left should be OR expression (from parentheses)
                        match left.as_ref() {
                            Expr::Logical { op: inner_op, .. } => {
                                assert_eq!(*inner_op, LogicalOp::Or);
                            }
                            _ => panic!("expected Logical OR in parentheses"),
                        }
                    }
                    _ => panic!("expected Logical AND expression"),
                },
                _ => panic!("expected Filter selector"),
            },
            _ => panic!("expected Child segment"),
        }
    }

    #[test]
    fn test_parse_filter_current_node_only() {
        let path = Parser::parse("$[?@]").unwrap();
        match &path.segments[0] {
            Segment::Child(selectors) => match &selectors[0] {
                Selector::Filter(expr) => {
                    assert_eq!(**expr, Expr::CurrentNode);
                }
                _ => panic!("expected Filter selector"),
            },
            _ => panic!("expected Child segment"),
        }
    }
}
