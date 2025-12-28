//! Parser for JSONPath queries

use crate::ast::{CompOp, Expr, JsonPath, Literal, LogicalOp, Segment, Selector};
use crate::lexer::{Lexer, LexerError, Token, TokenKind};

/// RFC 9535: Functions that return LogicalType (cannot be used in comparisons)
const LOGICAL_TYPE_FUNCTIONS: &[&str] = &["match", "search"];

/// RFC 9535: Functions that return ComparisonType (must be compared, cannot be existence test)
const COMPARISON_TYPE_FUNCTIONS: &[&str] = &["count", "length", "value"];

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
        // RFC 9535: JSONPath must start with '$', no leading whitespace allowed
        if let Some(first_char) = input.chars().next()
            && first_char.is_whitespace()
        {
            return Err(ParseError {
                message: "leading whitespace is not allowed".to_string(),
                position: 0,
            });
        }

        // RFC 9535: No trailing whitespace allowed
        if let Some(last_char) = input.chars().last()
            && last_char.is_whitespace()
        {
            return Err(ParseError {
                message: "trailing whitespace is not allowed".to_string(),
                position: input.len() - 1,
            });
        }

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
                let dot_pos = self.current_position();
                self.advance();
                // RFC 9535: No whitespace allowed after '..'
                if self.current_position() != dot_pos + 2 {
                    return Err(ParseError {
                        message: "whitespace not allowed after '..'".to_string(),
                        position: dot_pos + 2,
                    });
                }
                let selectors = self.parse_selectors_after_dot()?;
                Ok(Segment::Descendant(selectors))
            }
            Some(TokenKind::Dot) => {
                let dot_pos = self.current_position();
                self.advance();
                // RFC 9535: No whitespace allowed after '.'
                if self.current_position() != dot_pos + 1 {
                    return Err(ParseError {
                        message: "whitespace not allowed after '.'".to_string(),
                        position: dot_pos + 1,
                    });
                }
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
        // RFC 9535: Keywords are valid as property names in dot notation
        if let Some(name) = self.current_kind().and_then(Self::keyword_to_property_name) {
            self.advance();
            return Ok(vec![Selector::Name(name.to_string())]);
        }
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
            Some(TokenKind::Number(_, _)) | Some(TokenKind::Colon) => self.parse_index_or_slice(),
            Some(TokenKind::Question) => {
                self.advance(); // consume '?'
                let expr = self.parse_expression()?;
                // RFC 9535: Literal alone is not allowed as filter expression
                if matches!(expr, Expr::Literal(_)) {
                    return Err(ParseError {
                        message: "filter expression cannot be a literal alone".to_string(),
                        position: self.current_position(),
                    });
                }
                // RFC 9535: ComparisonType functions (count, length, value) must be compared
                // They cannot be used as standalone existence tests
                if let Expr::FunctionCall { name, .. } = &expr
                    && COMPARISON_TYPE_FUNCTIONS.contains(&name.as_str())
                {
                    return Err(ParseError {
                        message: format!(
                            "function '{}' returns a value that must be compared",
                            name
                        ),
                        position: self.current_position(),
                    });
                }
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
        let start = self.try_parse_index_number()?;

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

        let end = self.try_parse_index_number()?;

        let step = if self.current_kind() == Some(&TokenKind::Colon) {
            self.advance(); // consume second ':'
            self.try_parse_index_number()?
        } else {
            None
        };

        Ok(Selector::Slice { start, end, step })
    }

    /// RFC 9535 exact integer range: -(2^53-1) to (2^53-1)
    const RFC9535_MIN_INT: i64 = -9007199254740991; // -(2^53 - 1)
    const RFC9535_MAX_INT: i64 = 9007199254740991; // 2^53 - 1

    /// Try to parse a number for index/slice selector
    /// Returns Ok(Some(n)) if valid integer, Ok(None) if no number token, Err if invalid
    fn try_parse_index_number(&mut self) -> Result<Option<i64>, ParseError> {
        if let Some(TokenKind::Number(n, has_decimal_or_exp)) = self.current_kind() {
            let n = *n;
            let has_decimal_or_exp = *has_decimal_or_exp;
            let pos = self.current_position();

            // RFC 9535: -0 is not valid for index/slice selectors
            if n == 0.0 && n.is_sign_negative() {
                return Err(ParseError {
                    message: "-0 is not valid for index selector".to_string(),
                    position: pos,
                });
            }

            // RFC 9535: Index must be written as integer (no decimal point or exponent)
            if has_decimal_or_exp {
                return Err(ParseError {
                    message: "index must be an integer, not a decimal".to_string(),
                    position: pos,
                });
            }

            // Check RFC 9535 exact integer range
            if n < Self::RFC9535_MIN_INT as f64 || n > Self::RFC9535_MAX_INT as f64 {
                return Err(ParseError {
                    message: "index out of range (must be between -(2^53-1) and 2^53-1)"
                        .to_string(),
                    position: pos,
                });
            }

            self.advance();
            Ok(Some(n as i64))
        } else {
            Ok(None)
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

    /// Convert keyword TokenKind to property name string
    /// RFC 9535: Keywords (true, false, null) are valid as property names
    fn keyword_to_property_name(kind: &TokenKind) -> Option<&'static str> {
        match kind {
            TokenKind::True => Some("true"),
            TokenKind::False => Some("false"),
            TokenKind::Null => Some("null"),
            _ => None,
        }
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
            let op_pos = self.current_position();
            self.advance(); // consume '||'
            let right = self.parse_and_expression()?;

            // RFC 9535: Logical operators require LogicalType operands (not bare literals)
            Self::validate_logical_operand(&left, op_pos)?;
            Self::validate_logical_operand(&right, op_pos)?;

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
            let op_pos = self.current_position();
            self.advance(); // consume '&&'
            let right = self.parse_comparison_expression()?;

            // RFC 9535: Logical operators require LogicalType operands (not bare literals)
            Self::validate_logical_operand(&left, op_pos)?;
            Self::validate_logical_operand(&right, op_pos)?;

            left = Expr::Logical {
                left: Box::new(left),
                op: LogicalOp::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Validate that an expression is a valid LogicalType operand for && or ||
    /// RFC 9535: Bare literals are not allowed as operands of logical operators
    fn validate_logical_operand(expr: &Expr, pos: usize) -> Result<(), ParseError> {
        if matches!(expr, Expr::Literal(_)) {
            return Err(ParseError {
                message: "literal cannot be used as operand of logical operator".to_string(),
                position: pos,
            });
        }
        Ok(())
    }

    /// Check if an expression is a singular query (returns at most one value)
    /// RFC 9535 requires comparison operands to be singular queries
    fn is_singular_query(expr: &Expr) -> bool {
        match expr {
            Expr::Path { segments, .. } => segments.iter().all(|seg| match seg {
                Segment::Child(selectors) => {
                    selectors.len() == 1
                        && matches!(&selectors[0], Selector::Name(_) | Selector::Index(_))
                }
                Segment::Descendant(_) => false,
            }),
            Expr::CurrentNode | Expr::RootNode => true,
            Expr::Literal(_) => true,
            Expr::FunctionCall { .. } => true,
            _ => false,
        }
    }

    /// Check if an expression is a LogicalType function (match, search)
    /// Returns the function name if it is, None otherwise
    fn get_logical_type_function_name(expr: &Expr) -> Option<&str> {
        if let Expr::FunctionCall { name, .. } = expr
            && LOGICAL_TYPE_FUNCTIONS.contains(&name.as_str())
        {
            return Some(name.as_str());
        }
        None
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
            let op_pos = self.current_position();
            self.advance(); // consume operator
            let right = self.parse_unary_expression()?;

            // RFC 9535: Both sides of comparison must be singular queries
            if !Self::is_singular_query(&left) {
                return Err(ParseError {
                    message: "non-singular query not allowed in comparison".to_string(),
                    position: op_pos,
                });
            }
            if !Self::is_singular_query(&right) {
                return Err(ParseError {
                    message: "non-singular query not allowed in comparison".to_string(),
                    position: op_pos,
                });
            }

            // RFC 9535: LogicalType functions (match, search) cannot be compared
            for expr in [&left, &right] {
                if let Some(name) = Self::get_logical_type_function_name(expr) {
                    return Err(ParseError {
                        message: format!(
                            "function '{}' returns LogicalType and cannot be compared",
                            name
                        ),
                        position: op_pos,
                    });
                }
            }

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
            Some(TokenKind::Number(n, _)) => {
                self.advance();
                Ok(Expr::Literal(Literal::Number(n)))
            }
            Some(TokenKind::String(s)) => {
                self.advance();
                Ok(Expr::Literal(Literal::String(s)))
            }
            Some(TokenKind::Ident(name)) => {
                let ident_pos = self.current_position();
                let ident_len = name.len();
                self.advance();
                // Check if this is a function call
                if self.current_kind() == Some(&TokenKind::ParenOpen) {
                    // RFC 9535: No whitespace allowed between function name and '('
                    if self.current_position() != ident_pos + ident_len {
                        return Err(ParseError {
                            message: "whitespace not allowed between function name and '('"
                                .to_string(),
                            position: ident_pos + ident_len,
                        });
                    }
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
                let dot_pos = self.current_position();
                self.advance();
                // RFC 9535: No whitespace allowed after '..'
                if self.current_position() != dot_pos + 2 {
                    return Err(ParseError {
                        message: "whitespace not allowed after '..'".to_string(),
                        position: dot_pos + 2,
                    });
                }
                let selectors = self.parse_filter_selectors_after_dot()?;
                Ok(Segment::Descendant(selectors))
            }
            Some(TokenKind::Dot) => {
                let dot_pos = self.current_position();
                self.advance();
                // RFC 9535: No whitespace allowed after '.'
                if self.current_position() != dot_pos + 1 {
                    return Err(ParseError {
                        message: "whitespace not allowed after '.'".to_string(),
                        position: dot_pos + 1,
                    });
                }
                let selectors = self.parse_filter_selectors_after_dot()?;
                Ok(Segment::Child(selectors))
            }
            Some(TokenKind::BracketOpen) => {
                self.advance(); // consume '['
                let mut selectors = Vec::new();
                loop {
                    let selector = self.parse_filter_bracket_selector()?;
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
                        _ => {
                            return Err(ParseError {
                                message: "expected ',' or ']'".to_string(),
                                position: self.current_position(),
                            });
                        }
                    }
                }
                Ok(Segment::Child(selectors))
            }
            _ => Err(ParseError {
                message: "expected path segment".to_string(),
                position: self.current_position(),
            }),
        }
    }

    /// Parse selectors after '.' or '..' in filter path
    fn parse_filter_selectors_after_dot(&mut self) -> Result<Vec<Selector>, ParseError> {
        // RFC 9535: Keywords are valid as property names in dot notation
        if let Some(name) = self.current_kind().and_then(Self::keyword_to_property_name) {
            self.advance();
            return Ok(vec![Selector::Name(name.to_string())]);
        }
        match self.current_kind().cloned() {
            Some(TokenKind::Ident(name)) => {
                self.advance();
                Ok(vec![Selector::Name(name)])
            }
            Some(TokenKind::Wildcard) => {
                self.advance();
                Ok(vec![Selector::Wildcard])
            }
            // RFC 9535: Bracket selectors can follow '.' or '..' (e.g., $..['key'])
            Some(TokenKind::BracketOpen) => {
                self.advance(); // consume '['
                let mut selectors = Vec::new();
                loop {
                    let selector = self.parse_filter_bracket_selector()?;
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
                        _ => {
                            return Err(ParseError {
                                message: "expected ',' or ']'".to_string(),
                                position: self.current_position(),
                            });
                        }
                    }
                }
                Ok(selectors)
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

    /// Parse a bracket selector within filter path (name, index, wildcard, slice, or nested filter)
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
            Some(TokenKind::Number(_, _)) | Some(TokenKind::Colon) => self.parse_index_or_slice(),
            Some(TokenKind::Question) => {
                // Nested filter expression: [?expr]
                self.advance(); // consume '?'
                let expr = self.parse_expression()?;
                // RFC 9535: Literal alone is not allowed as filter expression
                if matches!(expr, Expr::Literal(_)) {
                    return Err(ParseError {
                        message: "filter expression cannot be a literal alone".to_string(),
                        position: self.current_position(),
                    });
                }
                Ok(Selector::Filter(Box::new(expr)))
            }
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
        let func_pos = self.current_position();

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

        // Validate function parameters per RFC 9535
        self.validate_function_params(&name, &args, func_pos)?;

        Ok(Expr::FunctionCall { name, args })
    }

    /// Check if an expression is a query (NodesType) - @ or $ based path
    fn is_nodes_type(expr: &Expr) -> bool {
        matches!(expr, Expr::CurrentNode | Expr::RootNode | Expr::Path { .. })
    }

    /// Check if an expression is ValueType (singular query or literal)
    /// RFC 9535: ValueType can be used where a single value is expected
    fn is_value_type(expr: &Expr) -> bool {
        match expr {
            Expr::Literal(_) => true,
            Expr::CurrentNode | Expr::RootNode => true, // Bare @ or $ is singular
            Expr::Path { segments, .. } => {
                // Path must be singular: only single name/index selectors, no descendants
                segments.iter().all(|seg| match seg {
                    Segment::Child(selectors) => {
                        selectors.len() == 1
                            && matches!(&selectors[0], Selector::Name(_) | Selector::Index(_))
                    }
                    Segment::Descendant(_) => false,
                })
            }
            // FunctionCalls that return ValueType are allowed (ComparisonType functions)
            Expr::FunctionCall { name, .. } => COMPARISON_TYPE_FUNCTIONS.contains(&name.as_str()),
            _ => false,
        }
    }

    /// Validate function parameter count and types per RFC 9535
    fn validate_function_params(
        &self,
        name: &str,
        args: &[Expr],
        pos: usize,
    ) -> Result<(), ParseError> {
        match name {
            // count(NodesType) - exactly 1 argument, must be a query (not literal)
            "count" => {
                if args.len() != 1 {
                    return Err(ParseError {
                        message: format!(
                            "function 'count' requires exactly 1 argument, got {}",
                            args.len()
                        ),
                        position: pos,
                    });
                }
                if !Self::is_nodes_type(&args[0]) {
                    return Err(ParseError {
                        message: "function 'count' requires a query argument (NodesType)"
                            .to_string(),
                        position: pos,
                    });
                }
            }
            // length(ValueType) - exactly 1 argument, must be singular query or literal
            "length" => {
                if args.len() != 1 {
                    return Err(ParseError {
                        message: format!(
                            "function 'length' requires exactly 1 argument, got {}",
                            args.len()
                        ),
                        position: pos,
                    });
                }
                // RFC 9535: length() argument must be ValueType (singular query or literal)
                if !Self::is_value_type(&args[0]) {
                    return Err(ParseError {
                        message: "function 'length' requires a singular query or literal argument"
                            .to_string(),
                        position: pos,
                    });
                }
            }
            // match(ValueType, ValueType) - exactly 2 arguments, both must be ValueType
            "match" => {
                if args.len() != 2 {
                    return Err(ParseError {
                        message: format!(
                            "function 'match' requires exactly 2 arguments, got {}",
                            args.len()
                        ),
                        position: pos,
                    });
                }
                // RFC 9535: Both arguments must be ValueType (singular query or literal)
                if !Self::is_value_type(&args[0]) {
                    return Err(ParseError {
                        message:
                            "function 'match' first argument must be a singular query or literal"
                                .to_string(),
                        position: pos,
                    });
                }
                if !Self::is_value_type(&args[1]) {
                    return Err(ParseError {
                        message:
                            "function 'match' second argument must be a singular query or literal"
                                .to_string(),
                        position: pos,
                    });
                }
            }
            // search(ValueType, ValueType) - exactly 2 arguments, both must be ValueType
            "search" => {
                if args.len() != 2 {
                    return Err(ParseError {
                        message: format!(
                            "function 'search' requires exactly 2 arguments, got {}",
                            args.len()
                        ),
                        position: pos,
                    });
                }
                // RFC 9535: Both arguments must be ValueType (singular query or literal)
                if !Self::is_value_type(&args[0]) {
                    return Err(ParseError {
                        message:
                            "function 'search' first argument must be a singular query or literal"
                                .to_string(),
                        position: pos,
                    });
                }
                if !Self::is_value_type(&args[1]) {
                    return Err(ParseError {
                        message:
                            "function 'search' second argument must be a singular query or literal"
                                .to_string(),
                        position: pos,
                    });
                }
            }
            // value(NodesType) - exactly 1 argument, must be a query (not literal)
            "value" => {
                if args.len() != 1 {
                    return Err(ParseError {
                        message: format!(
                            "function 'value' requires exactly 1 argument, got {}",
                            args.len()
                        ),
                        position: pos,
                    });
                }
                if !Self::is_nodes_type(&args[0]) {
                    return Err(ParseError {
                        message: "function 'value' requires a query argument (NodesType)"
                            .to_string(),
                        position: pos,
                    });
                }
            }
            // RFC 9535: Only the 5 defined functions are allowed
            _ => {
                return Err(ParseError {
                    message: format!("unknown function '{}'", name),
                    position: pos,
                });
            }
        }
        Ok(())
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
                            assert_eq!(**right, Expr::Literal(Literal::Number(10.0)));
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
                        assert_eq!(**right, Expr::Literal(Literal::Number(0.0)));
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

    // ========== Whitespace Validation Tests ==========

    #[test]
    fn test_reject_leading_whitespace() {
        let result = Parser::parse(" $");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("leading whitespace"));
        assert_eq!(err.position, 0);
    }

    #[test]
    fn test_reject_trailing_whitespace() {
        let result = Parser::parse("$ ");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("trailing whitespace"));
    }

    #[test]
    fn test_reject_both_leading_and_trailing_whitespace() {
        let result = Parser::parse(" $ ");
        assert!(result.is_err());
        // Leading whitespace is checked first
        let err = result.unwrap_err();
        assert!(err.message.contains("leading whitespace"));
    }

    #[test]
    fn test_reject_tab_whitespace() {
        let result = Parser::parse("\t$");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("leading whitespace"));
    }

    #[test]
    fn test_reject_newline_whitespace() {
        let result = Parser::parse("$\n");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("trailing whitespace"));
    }

    // ========== Keyword as Property Name Tests ==========

    #[test]
    fn test_keyword_true_as_property() {
        let path = Parser::parse("$.true").unwrap();
        assert_eq!(path.segments.len(), 1);
        assert_eq!(
            path.segments[0],
            Segment::Child(vec![Selector::Name("true".to_string())])
        );
    }

    #[test]
    fn test_keyword_false_as_property() {
        let path = Parser::parse("$.false").unwrap();
        assert_eq!(path.segments.len(), 1);
        assert_eq!(
            path.segments[0],
            Segment::Child(vec![Selector::Name("false".to_string())])
        );
    }

    #[test]
    fn test_keyword_null_as_property() {
        let path = Parser::parse("$.null").unwrap();
        assert_eq!(path.segments.len(), 1);
        assert_eq!(
            path.segments[0],
            Segment::Child(vec![Selector::Name("null".to_string())])
        );
    }

    #[test]
    fn test_keyword_in_filter_path() {
        // $.items[?@.null] should parse @.null as a path to property "null"
        let path = Parser::parse("$[?@.true]").unwrap();
        match &path.segments[0] {
            Segment::Child(selectors) => match &selectors[0] {
                Selector::Filter(expr) => match expr.as_ref() {
                    Expr::Path { segments, .. } => {
                        assert_eq!(
                            segments[0],
                            Segment::Child(vec![Selector::Name("true".to_string())])
                        );
                    }
                    _ => panic!("expected Path expression"),
                },
                _ => panic!("expected Filter selector"),
            },
            _ => panic!("expected Child segment"),
        }
    }

    // ========== Nested Filter Tests ==========

    #[test]
    fn test_parse_nested_filter() {
        // $[?@[?@.a]] should parse successfully
        let path = Parser::parse("$[?@[?@.a]]").unwrap();
        assert_eq!(path.segments.len(), 1);
        match &path.segments[0] {
            Segment::Child(selectors) => match &selectors[0] {
                Selector::Filter(outer_expr) => match outer_expr.as_ref() {
                    Expr::Path { start, segments } => {
                        assert_eq!(**start, Expr::CurrentNode);
                        assert_eq!(segments.len(), 1);
                        // The inner segment should contain a nested filter
                        match &segments[0] {
                            Segment::Child(inner_selectors) => {
                                assert!(matches!(inner_selectors[0], Selector::Filter(_)));
                            }
                            _ => panic!("expected Child segment"),
                        }
                    }
                    _ => panic!("expected Path expression"),
                },
                _ => panic!("expected Filter selector"),
            },
            _ => panic!("expected Child segment"),
        }
    }

    // ========== Function Type Validation Tests ==========

    #[test]
    fn test_comparison_type_function_in_existence_test() {
        // RFC 9535: count/length/value return ComparisonType, cannot be used as existence test
        let result = Parser::parse("$[?count(@.x)]");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("returns a value that must be compared")
        );

        let result = Parser::parse("$[?length(@.x)]");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("returns a value that must be compared")
        );

        let result = Parser::parse("$[?value(@.x)]");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("returns a value that must be compared")
        );
    }

    #[test]
    fn test_comparison_type_function_in_comparison_ok() {
        // ComparisonType functions CAN be used in comparisons
        assert!(Parser::parse("$[?count(@.x) > 0]").is_ok());
        assert!(Parser::parse("$[?length(@.x) == 5]").is_ok());
        assert!(Parser::parse("$[?value(@.x) != null]").is_ok());
    }

    #[test]
    fn test_logical_type_function_in_comparison() {
        // RFC 9535: match/search return LogicalType, cannot be compared
        let result = Parser::parse("$[?match(@.x, \"a\") == true]");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("returns LogicalType and cannot be compared")
        );

        let result = Parser::parse("$[?search(@.x, \"a\") == true]");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("returns LogicalType and cannot be compared")
        );
    }

    #[test]
    fn test_logical_type_function_on_right_side() {
        // LogicalType validation applies to right side too
        let result = Parser::parse("$[?true == match(@.x, \"a\")]");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("returns LogicalType and cannot be compared")
        );
    }

    #[test]
    fn test_logical_type_function_in_existence_test_ok() {
        // LogicalType functions CAN be used as existence tests
        assert!(Parser::parse("$[?match(@.x, \"a\")]").is_ok());
        assert!(Parser::parse("$[?search(@.x, \"a\")]").is_ok());
    }

    #[test]
    fn test_match_search_value_type_validation() {
        // RFC 9535: match/search require ValueType arguments (singular query or literal)

        // Non-singular query (wildcard) as first argument - should fail
        let result = Parser::parse("$[?match(@[*], \"a\")]");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("first argument must be a singular query or literal")
        );

        let result = Parser::parse("$[?search(@[*], \"a\")]");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("first argument must be a singular query or literal")
        );

        // Non-singular query (descendant) as first argument - should fail
        let result = Parser::parse("$[?match(@..x, \"a\")]");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("first argument must be a singular query or literal")
        );

        // Singular query and literal - should pass
        assert!(Parser::parse("$[?match(@.x, \"a\")]").is_ok());
        assert!(Parser::parse("$[?search(@.name, \"pattern\")]").is_ok());
        assert!(Parser::parse("$[?match(\"test\", \"t.*\")]").is_ok());
    }

    #[test]
    fn test_unknown_function_rejected() {
        // RFC 9535: Only count, length, match, search, value are defined
        let result = Parser::parse("$[?first(@.x)]");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("unknown function 'first'")
        );

        let result = Parser::parse("$[?last(@.x)]");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("unknown function 'last'")
        );

        let result = Parser::parse("$[?min(@.x)]");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("unknown function 'min'")
        );

        // Known functions should still work
        assert!(Parser::parse("$[?count(@.x) > 0]").is_ok());
        assert!(Parser::parse("$[?length(@.x) > 0]").is_ok());
        assert!(Parser::parse("$[?match(@.x, \"a\")]").is_ok());
        assert!(Parser::parse("$[?search(@.x, \"a\")]").is_ok());
        assert!(Parser::parse("$[?value(@.x) == 1]").is_ok());
    }
}
