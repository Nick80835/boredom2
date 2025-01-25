#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token {
    RawIdentifier {
        src_line: usize,
        value: String,
    },
    IntegerLiteral {
        value: u32,
    },
    StringLiteral {
        value: String,
    },
    Symbol {
        src_line: usize,
        value: char,
    },
    Whitespace,
    Comment,
    EOF,
    // special tokens, returned by post_process
    If {
        src_line: usize,
    },
    While {
        src_line: usize,
    },
    Else,
    ScopeOpen {
        src_line: usize,
    },
    ScopeClose {
        src_line: usize,
    },
    ParensOpen,
    ParensClose,
    Assign,
    Is,
    ArrayAccess,
    Question,
    Equals,
    NotEquals,
    MoreThan,
    LessThan,
    MoreThanOrEquals,
    LessThanOrEquals,
    BoolTrue,
    BoolFalse,
    Plus,
    Minus,
    PlusEquals,
    MinusEquals,
    Alloc {
        src_line: usize,
    },
    Set {
        src_line: usize,
    },
    ArrayOpen,
    ArrayClose,
    Print {
        src_line: usize,
    },
    ReadLine {
        src_line: usize,
    },
    LineEnd,
    Label {
        src_line: usize,
    },
    Jump {
        src_line: usize,
    },
    Variable {
        src_line: usize,
        name: String,
    },
}

pub struct Tokenizer {
    lines: Vec<String>,
    line_idx: usize,
    char_idx: usize,
}

impl Tokenizer {
    pub fn init(lines: Vec<String>) -> Self { Self { lines, line_idx: 0, char_idx: 0 } }
    fn line_idx_in_bounds(&self) -> bool { self.line_idx < self.lines.len() }
    fn char_idx_in_bounds(&self) -> bool { self.char_idx < self.get_current_line().len() }
    fn get_current_line(&self) -> &String { &self.lines[self.line_idx] }
    fn get_current_char(&self) -> char { self.get_current_line().chars().collect::<Vec<char>>()[self.char_idx] }
    fn special_symbols() -> Vec<char> {
        vec!['?', '=', '{', '}', '>', '<', ';', '+', '-', ':', '[', ']', '|', '(', ')']
    }

    pub fn next_token(&mut self) -> Token {
        if !self.char_idx_in_bounds() {
            self.char_idx = 0;
            self.line_idx += 1;

            while self.line_idx_in_bounds() && self.get_current_line().is_empty() {
                self.line_idx += 1
            }
        }
        if !self.line_idx_in_bounds() {
            return Token::EOF;
        }

        let this_char = self.get_current_char();

        if this_char.is_ascii_digit() {
            return self.consume_integer();
        } else if this_char.is_ascii_alphabetic() {
            // identifiers can only start with a letter
            return self.consume_identifier();
        } else if this_char == '"' {
            return self.consume_string_literal();
        } else if this_char.is_ascii_whitespace() {
            // coalesce whitespace
            return self.consume_whitespace();
        } else if this_char == '#' {
            // comments
            return self.consume_comment();
        } else if Tokenizer::special_symbols().contains(&this_char) {
            self.char_idx += 1;
            return Token::Symbol { src_line: self.line_idx + 1, value: this_char };
        } else {
            panic!("Unknown char '{}' at line {}, exiting.", this_char, self.line_idx + 1)
        }
    }

    fn unraw_token(token: Token) -> Token {
        match &token {
            Token::RawIdentifier { src_line, value } => {
                match value.as_str() {
                    "if" => Token::If { src_line: *src_line },
                    "while" => Token::While { src_line: *src_line },
                    "is" => Token::Is,
                    "else" => Token::Else,
                    "alloc" => Token::Alloc { src_line: *src_line },
                    "set" => Token::Set { src_line: *src_line },
                    "print" => Token::Print { src_line: *src_line },
                    "readln" => Token::ReadLine { src_line: *src_line },
                    "true" => Token::BoolTrue,
                    "false" => Token::BoolFalse,
                    "jump" => Token::Jump { src_line: *src_line },
                    _ => Token::Variable{ src_line: *src_line, name: value.to_string() },
                }
            }
            Token::Symbol { src_line, value } => {
                match value {
                    '?' => Token::Question,
                    '=' => Token::Assign,
                    '{' => Token::ScopeOpen { src_line: *src_line },
                    '}' => Token::ScopeClose { src_line: *src_line },
                    '>' => Token::MoreThan,
                    '<' => Token::LessThan,
                    ';' => Token::LineEnd,
                    '+' => Token::Plus,
                    '-' => Token::Minus,
                    ':' => Token::Label { src_line: *src_line },
                    '[' => Token::ArrayOpen,
                    ']' => Token::ArrayClose,
                    '|' => Token::ArrayAccess,
                    '(' => Token::ParensOpen,
                    ')' => Token::ParensClose,
                    _ => token,
                }
            }
            _ => token
        }
    }

    pub fn post_process(tokens: Vec<Token>) -> Vec<Token> {
        let mut out_tokens: Vec<Token> = vec![];

        // remove whitespace and coalesce some tokens
        for (token_idx, token) in tokens.iter().enumerate() {
            let token = token.clone();
            if token == Token::Whitespace || token == Token::Comment { continue; }

            // coalesce *= to equivalent comparison tokens
            if let Token::Symbol { src_line: _, value: '=' } = token {
                if token_idx < 1 {
                    out_tokens.push(Tokenizer::unraw_token(token));
                } else {
                    match &tokens[token_idx - 1] { // previous token
                        // comparison
                        Token::Symbol { src_line: _, value: '=' } => {
                            out_tokens.truncate(out_tokens.len() - 1);
                            out_tokens.push(Token::Equals);
                        }
                        Token::Symbol { src_line: _, value: '!' } => {
                            out_tokens.truncate(out_tokens.len() - 1);
                            out_tokens.push(Token::NotEquals);
                        }
                        Token::Symbol { src_line: _, value: '>' } => {
                            out_tokens.truncate(out_tokens.len() - 1);
                            out_tokens.push(Token::MoreThanOrEquals);
                        }
                        Token::Symbol { src_line: _, value: '<' } => {
                            out_tokens.truncate(out_tokens.len() - 1);
                            out_tokens.push(Token::LessThanOrEquals);
                        }
                        // math
                        Token::Symbol { src_line: _, value: '+' } => {
                            out_tokens.truncate(out_tokens.len() - 1);
                            out_tokens.push(Token::PlusEquals);
                        }
                        Token::Symbol { src_line: _, value: '-' } => {
                            out_tokens.truncate(out_tokens.len() - 1);
                            out_tokens.push(Token::MinusEquals);
                        }
                        _ => {
                            out_tokens.push(Tokenizer::unraw_token(token));
                        }
                    }
                }
            } else {
                out_tokens.push(Tokenizer::unraw_token(token));
            }
        }

        // validate scope closures, not wholly necessary here but for now it helps
        let mut scope_open_idxs: Vec<usize> = vec![];

        for (token_idx, token) in out_tokens.iter().enumerate() {
            match token {
                Token::ScopeOpen { src_line: _ } => {
                    scope_open_idxs.push(token_idx);
                }
                Token::ScopeClose { src_line: _ } => {
                    scope_open_idxs.pop();
                }
                _ => {
                    // don't care
                }
            }
        }

        assert_eq!(scope_open_idxs.len(), 0);
        return out_tokens;
    }

    fn consume_integer(&mut self) -> Token {
        let mut digit_str = String::new();

        while self.char_idx_in_bounds() && self.get_current_char().is_ascii_digit() {
            digit_str.push(self.get_current_char());
            self.char_idx += 1
        }

        Token::IntegerLiteral {
            value: u32::from_str_radix(&digit_str, 10).unwrap()
        }
    }

    fn consume_identifier(&mut self) -> Token {
        let mut identifier_str = String::new();

        // identifiers may contain a number, only the start needs to be a letter
        while self.char_idx_in_bounds() && self.get_current_char().is_ascii_alphanumeric() {
            identifier_str.push(self.get_current_char());
            self.char_idx += 1
        }

        Token::RawIdentifier { src_line: self.line_idx + 1 , value: identifier_str }
    }

    fn consume_string_literal(&mut self) -> Token {
        let mut literal_str = String::new();
        self.char_idx += 1; // go into bounds of string

        while self.char_idx_in_bounds() && self.get_current_char() != '"' {
            literal_str.push(self.get_current_char());
            self.char_idx += 1
        }

        self.char_idx += 1; // leave string bounds
        Token::StringLiteral {
            value: literal_str
        }
    }

    fn consume_whitespace(&mut self) -> Token {
        while self.char_idx_in_bounds() && self.get_current_char().is_ascii_whitespace() {
            self.char_idx += 1
        }

        Token::Whitespace
    }

    fn consume_comment(&mut self) -> Token {
        while self.char_idx_in_bounds() {
            self.char_idx += 1
        }

        Token::Comment
    }
}
