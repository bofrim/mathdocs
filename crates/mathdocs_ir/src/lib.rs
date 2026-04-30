use mathdocs_ast::{Diagnostic, DiagnosticSeverity, TextRange};
use mathdocs_metadata::expand_symbol_lookups;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Name(String),
    Literal(String),
    StringLiteral(String),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Compare {
        op: CompareOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    Attribute {
        value: Box<Expr>,
        attr: String,
    },
    Subscript {
        value: Box<Expr>,
        indices: Vec<Expr>,
    },
    Group(Box<Expr>),
    Unsupported(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Plus,
    Minus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    MatMul,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompareOp {
    Eq,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    Assignment { target: Expr, value: Expr },
    Expr(Expr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoweredStatement {
    pub statement: Statement,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn lower_statement(source: &str, range: TextRange) -> LoweredStatement {
    let expanded = expand_symbol_lookups(source);
    let clean = strip_comment(&expanded).trim();
    let mut diagnostics = Vec::new();

    if let Some((left, right)) = split_top_level_assignment(clean) {
        let target = parse_expr_or_unsupported(left, range, &mut diagnostics);
        let value = parse_expr_or_unsupported(right, range, &mut diagnostics);
        return LoweredStatement {
            statement: Statement::Assignment { target, value },
            diagnostics,
        };
    }

    if let Some((left, right)) = split_top_level_compare(clean) {
        let left = parse_expr_or_unsupported(left, range, &mut diagnostics);
        let right = parse_expr_or_unsupported(right, range, &mut diagnostics);
        return LoweredStatement {
            statement: Statement::Expr(Expr::Compare {
                op: CompareOp::Eq,
                left: Box::new(left),
                right: Box::new(right),
            }),
            diagnostics,
        };
    }

    let expr = parse_expr_or_unsupported(clean, range, &mut diagnostics);
    LoweredStatement {
        statement: Statement::Expr(expr),
        diagnostics,
    }
}

fn parse_expr_or_unsupported(
    source: &str,
    range: TextRange,
    diagnostics: &mut Vec<Diagnostic>,
) -> Expr {
    match Parser::new(source).parse_expression() {
        Ok(expr) => expr,
        Err(message) => {
            diagnostics.push(Diagnostic {
                code: "mathdocs::unsupported".to_string(),
                message,
                range,
                severity: DiagnosticSeverity::Warning,
            });
            Expr::Unsupported(source.to_string())
        }
    }
}

fn strip_comment(source: &str) -> &str {
    let bytes = source.as_bytes();
    let mut quote: Option<u8> = None;
    let mut idx = 0usize;
    while idx < bytes.len() {
        let byte = bytes[idx];
        if let Some(q) = quote {
            if byte == b'\\' {
                idx += 2;
                continue;
            }
            if byte == q {
                quote = None;
            }
            idx += 1;
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = Some(byte),
            b'#' => return &source[..idx],
            _ => {}
        }
        idx += 1;
    }
    source
}

fn split_top_level_assignment(source: &str) -> Option<(&str, &str)> {
    split_top_level_token(source, "=")
        .filter(|(left, right)| !left.ends_with(['=', '!', '<', '>']) && !right.starts_with('='))
}

fn split_top_level_compare(source: &str) -> Option<(&str, &str)> {
    split_top_level_token(source, "==")
}

fn split_top_level_token<'a>(source: &'a str, token: &str) -> Option<(&'a str, &'a str)> {
    let bytes = source.as_bytes();
    let token_bytes = token.as_bytes();
    let mut quote: Option<u8> = None;
    let mut depth = 0i32;
    let mut idx = 0usize;
    while idx + token_bytes.len() <= bytes.len() {
        let byte = bytes[idx];
        if let Some(q) = quote {
            if byte == b'\\' {
                idx += 2;
                continue;
            }
            if byte == q {
                quote = None;
            }
            idx += 1;
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = Some(byte),
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ if depth == 0 && &bytes[idx..idx + token_bytes.len()] == token_bytes => {
                return Some((source[..idx].trim(), source[idx + token.len()..].trim()));
            }
            _ => {}
        }
        idx += 1;
    }
    None
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Name(String),
    Number(String),
    Str(String),
    Plus,
    Minus,
    Star,
    Slash,
    Pow,
    At,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Dot,
    Comma,
    Colon,
    EqEq,
    Eof,
}

struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    idx: usize,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            idx: 0,
        }
    }

    fn next_token(&mut self) -> Result<Token, String> {
        while let Some(byte) = self.bytes.get(self.idx).copied() {
            if byte.is_ascii_whitespace() {
                self.idx += 1;
            } else {
                break;
            }
        }
        let Some(byte) = self.bytes.get(self.idx).copied() else {
            return Ok(Token::Eof);
        };
        match byte {
            b'+' => {
                self.idx += 1;
                Ok(Token::Plus)
            }
            b'-' => {
                self.idx += 1;
                Ok(Token::Minus)
            }
            b'*' if self.bytes.get(self.idx + 1) == Some(&b'*') => {
                self.idx += 2;
                Ok(Token::Pow)
            }
            b'*' => {
                self.idx += 1;
                Ok(Token::Star)
            }
            b'/' => {
                self.idx += 1;
                Ok(Token::Slash)
            }
            b'@' => {
                self.idx += 1;
                Ok(Token::At)
            }
            b'(' => {
                self.idx += 1;
                Ok(Token::LParen)
            }
            b')' => {
                self.idx += 1;
                Ok(Token::RParen)
            }
            b'[' => {
                self.idx += 1;
                Ok(Token::LBracket)
            }
            b']' => {
                self.idx += 1;
                Ok(Token::RBracket)
            }
            b'.' => {
                self.idx += 1;
                Ok(Token::Dot)
            }
            b',' => {
                self.idx += 1;
                Ok(Token::Comma)
            }
            b':' => {
                self.idx += 1;
                Ok(Token::Colon)
            }
            b'=' if self.bytes.get(self.idx + 1) == Some(&b'=') => {
                self.idx += 2;
                Ok(Token::EqEq)
            }
            b'\'' | b'"' => self.string(),
            b'0'..=b'9' => Ok(self.number()),
            _ if self.source[self.idx..]
                .chars()
                .next()
                .is_some_and(is_ident_start_char) =>
            {
                Ok(self.name())
            }
            _ => Err(format!(
                "unsupported token near {:?}",
                &self.source[self.idx..]
            )),
        }
    }

    fn name(&mut self) -> Token {
        let start = self.idx;
        let mut end = self.idx;
        for (offset, ch) in self.source[start..].char_indices() {
            if offset == 0 {
                end = start + ch.len_utf8();
                continue;
            }
            if is_ident_continue_char(ch) {
                end = start + offset + ch.len_utf8();
            } else {
                break;
            }
        }
        self.idx = end;
        Token::Name(self.source[start..self.idx].to_string())
    }

    fn number(&mut self) -> Token {
        let start = self.idx;
        self.idx += 1;
        while self
            .bytes
            .get(self.idx)
            .copied()
            .is_some_and(|b| b.is_ascii_digit() || b == b'.')
        {
            self.idx += 1;
        }
        Token::Number(self.source[start..self.idx].to_string())
    }

    fn string(&mut self) -> Result<Token, String> {
        let quote = self.bytes[self.idx];
        self.idx += 1;
        let mut value = String::new();
        while let Some(byte) = self.bytes.get(self.idx).copied() {
            if byte == quote {
                self.idx += 1;
                return Ok(Token::Str(value));
            }
            if byte == b'\\' {
                if let Some(next) = self.bytes.get(self.idx + 1).copied() {
                    value.push(next as char);
                    self.idx += 2;
                    continue;
                }
            }
            value.push(byte as char);
            self.idx += 1;
        }
        Err("unterminated string literal".to_string())
    }
}

fn is_ident_start_char(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_ident_continue_char(ch: char) -> bool {
    is_ident_start_char(ch) || ch.is_alphanumeric()
}

struct Parser {
    tokens: Vec<Token>,
    idx: usize,
}

impl Parser {
    fn new(source: &str) -> Self {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            match lexer.next_token() {
                Ok(Token::Eof) => {
                    tokens.push(Token::Eof);
                    break;
                }
                Ok(token) => tokens.push(token),
                Err(message) => {
                    tokens.push(Token::Name(format!("__unsupported_{message}")));
                    tokens.push(Token::Eof);
                    break;
                }
            }
        }
        Self { tokens, idx: 0 }
    }

    fn parse_expression(&mut self) -> Result<Expr, String> {
        let expr = self.parse_bp(0)?;
        if !matches!(
            self.peek(),
            Token::Eof | Token::RParen | Token::RBracket | Token::Comma
        ) {
            return Err(format!("unexpected token {:?}", self.peek()));
        }
        Ok(expr)
    }

    fn parse_bp(&mut self, min_bp: u8) -> Result<Expr, String> {
        let mut lhs = self.parse_prefix()?;
        loop {
            lhs = match self.peek() {
                Token::LParen => self.parse_call(lhs)?,
                Token::LBracket => self.parse_subscript(lhs)?,
                Token::Dot => {
                    self.bump();
                    let Token::Name(attr) = self.bump().clone() else {
                        return Err("expected attribute name after dot".to_string());
                    };
                    Expr::Attribute {
                        value: Box::new(lhs),
                        attr,
                    }
                }
                _ => break,
            };
        }

        loop {
            let Some((op, left_bp, right_bp)) = infix_binding_power(self.peek()) else {
                break;
            };
            if left_bp < min_bp {
                break;
            }
            self.bump();
            let rhs = self.parse_bp(right_bp)?;
            lhs = match op {
                InfixOp::Binary(op) => Expr::Binary {
                    op,
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                },
                InfixOp::Compare(op) => Expr::Compare {
                    op,
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                },
            };
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, String> {
        match self.bump().clone() {
            Token::Name(name) => Ok(Expr::Name(name)),
            Token::Number(number) => Ok(Expr::Literal(number)),
            Token::Str(value) => Ok(Expr::StringLiteral(value)),
            Token::Plus => Ok(Expr::Unary {
                op: UnaryOp::Plus,
                expr: Box::new(self.parse_bp(5)?),
            }),
            Token::Minus => Ok(Expr::Unary {
                op: UnaryOp::Minus,
                expr: Box::new(self.parse_bp(5)?),
            }),
            Token::LParen => {
                let expr = self.parse_bp(0)?;
                self.expect(Token::RParen)?;
                Ok(Expr::Group(Box::new(expr)))
            }
            token => Err(format!("expected expression, found {token:?}")),
        }
    }

    fn parse_call(&mut self, func: Expr) -> Result<Expr, String> {
        self.expect(Token::LParen)?;
        let mut args = Vec::new();
        if matches!(self.peek(), Token::RParen) {
            self.bump();
            return Ok(Expr::Call {
                func: Box::new(func),
                args,
            });
        }
        loop {
            args.push(self.parse_bp(0)?);
            match self.peek() {
                Token::Comma => {
                    self.bump();
                    if matches!(self.peek(), Token::RParen) {
                        break;
                    }
                }
                Token::RParen => break,
                other => return Err(format!("expected comma or right paren, found {other:?}")),
            }
        }
        self.expect(Token::RParen)?;
        Ok(Expr::Call {
            func: Box::new(func),
            args,
        })
    }

    fn parse_subscript(&mut self, value: Expr) -> Result<Expr, String> {
        self.expect(Token::LBracket)?;
        let mut indices = Vec::new();
        if matches!(self.peek(), Token::RBracket) {
            self.bump();
            return Ok(Expr::Subscript {
                value: Box::new(value),
                indices,
            });
        }
        loop {
            let mut expr = self.parse_bp(0)?;
            if matches!(self.peek(), Token::Colon) {
                self.bump();
                let left = match expr {
                    Expr::Literal(value) | Expr::Name(value) => value,
                    _ => String::new(),
                };
                let right = if !matches!(self.peek(), Token::Comma | Token::RBracket) {
                    match self.parse_bp(0)? {
                        Expr::Literal(value) | Expr::Name(value) => value,
                        _ => String::new(),
                    }
                } else {
                    String::new()
                };
                expr = Expr::Literal(format!("{left}:{right}"));
            }
            indices.push(expr);
            match self.peek() {
                Token::Comma => {
                    self.bump();
                    if matches!(self.peek(), Token::RBracket) {
                        break;
                    }
                }
                Token::RBracket => break,
                other => return Err(format!("expected comma or right bracket, found {other:?}")),
            }
        }
        self.expect(Token::RBracket)?;
        Ok(Expr::Subscript {
            value: Box::new(value),
            indices,
        })
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.idx).unwrap_or(&Token::Eof)
    }

    fn bump(&mut self) -> &Token {
        let idx = self.idx;
        self.idx += 1;
        self.tokens.get(idx).unwrap_or(&Token::Eof)
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        let actual = self.bump().clone();
        if actual == expected {
            Ok(())
        } else {
            Err(format!("expected {expected:?}, found {actual:?}"))
        }
    }
}

enum InfixOp {
    Binary(BinaryOp),
    Compare(CompareOp),
}

fn infix_binding_power(token: &Token) -> Option<(InfixOp, u8, u8)> {
    match token {
        Token::EqEq => Some((InfixOp::Compare(CompareOp::Eq), 1, 2)),
        Token::Plus => Some((InfixOp::Binary(BinaryOp::Add), 3, 4)),
        Token::Minus => Some((InfixOp::Binary(BinaryOp::Sub), 3, 4)),
        Token::Star => Some((InfixOp::Binary(BinaryOp::Mul), 5, 6)),
        Token::Slash => Some((InfixOp::Binary(BinaryOp::Div), 5, 6)),
        Token::At => Some((InfixOp::Binary(BinaryOp::MatMul), 5, 6)),
        Token::Pow => Some((InfixOp::Binary(BinaryOp::Pow), 8, 7)),
        _ => None,
    }
}

pub fn qualified_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Name(name) => Some(name.clone()),
        Expr::Attribute { value, attr } => Some(format!("{}.{}", qualified_name(value)?, attr)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowers_assignment_and_nested_calls() {
        let lowered = lower_statement(
            "z = abs(theta - mu) / sqrt(sigma)",
            TextRange { start: 0, end: 0 },
        );
        assert!(lowered.diagnostics.is_empty());
        assert!(matches!(lowered.statement, Statement::Assignment { .. }));
    }
}
