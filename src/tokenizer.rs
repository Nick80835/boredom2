#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token {
    RawIdentifier(String),
    IntegerLiteral(u32),
    StringLiteral(String),
    Symbol(char),
    Whitespace,
    Comment,
    EOF,
    // special tokens, returned by post_process
    If,
    While,
    Else,
    ScopeOpen,
    ScopeClose,
    ParensOpen,
    ParensClose,
    Assign,
    ArrayAccess,
    SubroutineCall,
    SubroutineDirect,
    SubroutineReturn,
    SubroutineDefine,
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
    Alloc,
    Set,
    ArrayOpen,
    ArrayClose,
    Comma,
    Print,
    ReadLine,
    LineEnd,
    Variable(String),
    LenAccess,
    PopAccess,
    PopFrontAccess,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct WrappedToken {
    pub token: Token,
    pub src_line: usize,
}

impl WrappedToken {
    pub fn from(token: Token) -> Self {
        Self { token, src_line: 0 }
    }
    pub fn from_with_line(token: Token, src_line: usize) -> Self {
        Self { token, src_line }
    }
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
        vec!['!', '?', '=', '{', '}', '>', '<', ';', '+', '-', '[', ']', '|', '(', ')', '.', ',']
    }

    pub fn next_token(&mut self) -> WrappedToken {
        if !self.char_idx_in_bounds() {
            self.char_idx = 0;
            self.line_idx += 1;

            while self.line_idx_in_bounds() && self.get_current_line().is_empty() {
                self.line_idx += 1
            }
        }
        if !self.line_idx_in_bounds() {
            return WrappedToken::from(Token::EOF);
        }

        let this_char = self.get_current_char();

        if this_char.is_ascii_digit() {
            return self.consume_integer();
        } else if this_char.is_ascii_alphabetic() || this_char == '_' {
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
            return WrappedToken::from_with_line(Token::Symbol(this_char), self.line_idx + 1);
        } else {
            panic!("Unknown char '{}' at line {}, exiting.", this_char, self.line_idx + 1)
        }
    }

    fn unraw_token(token: WrappedToken) -> WrappedToken {
        match &token.token {
            Token::RawIdentifier(value) => {
                match value.as_str() {
                    "if" => WrappedToken::from_with_line(Token::If, token.src_line),
                    "while" => WrappedToken::from_with_line(Token::While, token.src_line),
                    "else" => WrappedToken::from_with_line(Token::Else, token.src_line),
                    "alloc" => WrappedToken::from_with_line(Token::Alloc, token.src_line),
                    "set" => WrappedToken::from_with_line(Token::Set, token.src_line),
                    "print" => WrappedToken::from_with_line(Token::Print, token.src_line),
                    "readln" => WrappedToken::from_with_line(Token::ReadLine, token.src_line),
                    "true" => WrappedToken::from_with_line(Token::BoolTrue, token.src_line),
                    "false" => WrappedToken::from_with_line(Token::BoolFalse, token.src_line),
                    "call" => WrappedToken::from_with_line(Token::SubroutineCall, token.src_line),
                    "ret" => WrappedToken::from_with_line(Token::SubroutineReturn, token.src_line),
                    "sub" => WrappedToken::from_with_line(Token::SubroutineDefine, token.src_line),
                    _ => WrappedToken::from_with_line(Token::Variable(value.to_string()), token.src_line),
                }
            }
            Token::Symbol(value) => {
                match value {
                    '=' => WrappedToken::from_with_line(Token::Assign, token.src_line),
                    '{' => WrappedToken::from_with_line(Token::ScopeOpen, token.src_line),
                    '}' => WrappedToken::from_with_line(Token::ScopeClose, token.src_line),
                    '>' => WrappedToken::from_with_line(Token::MoreThan, token.src_line),
                    '<' => WrappedToken::from_with_line(Token::LessThan, token.src_line),
                    ';' => WrappedToken::from_with_line(Token::LineEnd, token.src_line),
                    '+' => WrappedToken::from_with_line(Token::Plus, token.src_line),
                    '-' => WrappedToken::from_with_line(Token::Minus, token.src_line),
                    '[' => WrappedToken::from_with_line(Token::ArrayOpen, token.src_line),
                    ']' => WrappedToken::from_with_line(Token::ArrayClose, token.src_line),
                    '|' => WrappedToken::from_with_line(Token::ArrayAccess, token.src_line),
                    '.' => WrappedToken::from_with_line(Token::LenAccess, token.src_line),
                    '(' => WrappedToken::from_with_line(Token::ParensOpen, token.src_line),
                    ')' => WrappedToken::from_with_line(Token::ParensClose, token.src_line),
                    ',' => WrappedToken::from_with_line(Token::Comma, token.src_line),
                    _ => token,
                }
            }
            _ => token
        }
    }

    pub fn post_process(tokens: Vec<WrappedToken>) -> Vec<WrappedToken> {
        let mut out_tokens: Vec<WrappedToken> = vec![];

        // remove whitespace and coalesce some tokens
        for (token_idx, token) in tokens.iter().enumerate() {
            let token = token.clone();

            if token.token == Token::Whitespace || token.token == Token::Comment {
                continue;
            }

            // coalesce *= to equivalent comparison tokens
            if token_idx < 1 {
                out_tokens.push(Tokenizer::unraw_token(token));
            } else {
                match &token.token {
                    Token::Symbol('=') => {
                        match &tokens[token_idx - 1].token { // get and replace previous token
                            // comparison
                            Token::Symbol('=') => {
                                out_tokens.truncate(out_tokens.len() - 1);
                                out_tokens.push(WrappedToken::from_with_line(Token::Equals, token.src_line));
                            }
                            Token::Symbol('!') => {
                                out_tokens.truncate(out_tokens.len() - 1);
                                out_tokens.push(WrappedToken::from_with_line(Token::NotEquals, token.src_line));
                            }
                            Token::Symbol('>') => {
                                out_tokens.truncate(out_tokens.len() - 1);
                                out_tokens.push(WrappedToken::from_with_line(Token::MoreThanOrEquals, token.src_line));
                            }
                            Token::Symbol('<') => {
                                out_tokens.truncate(out_tokens.len() - 1);
                                out_tokens.push(WrappedToken::from_with_line(Token::LessThanOrEquals, token.src_line));
                            }
                            // math
                            Token::Symbol('+') => {
                                out_tokens.truncate(out_tokens.len() - 1);
                                out_tokens.push(WrappedToken::from_with_line(Token::PlusEquals, token.src_line));
                            }
                            Token::Symbol('-') => {
                                out_tokens.truncate(out_tokens.len() - 1);
                                out_tokens.push(WrappedToken::from_with_line(Token::MinusEquals, token.src_line));
                            }
                            _ => {
                                out_tokens.push(Tokenizer::unraw_token(token));
                            }
                        }
                    }
                    Token::Symbol('>') => {
                        match &tokens[token_idx - 1].token { // get and replace previous token
                            // subroutine call
                            Token::Symbol('-') => {
                                out_tokens.truncate(out_tokens.len() - 1);
                                out_tokens.push(WrappedToken::from_with_line(Token::SubroutineDirect, token.src_line));
                            }
                            _ => {
                                out_tokens.push(Tokenizer::unraw_token(token));
                            }
                        }
                    }
                    Token::RawIdentifier(value) => {
                        match value.as_str() {
                            "pop" => {
                                match &tokens[token_idx - 1].token { // get and replace previous token
                                    Token::Symbol('.') => {
                                        // pop
                                        out_tokens.truncate(out_tokens.len() - 1);
                                        out_tokens.push(WrappedToken::from_with_line(Token::PopAccess, token.src_line));
                                    }
                                    _ => {
                                        // previous token was not '.' access, pop is a variable here
                                        out_tokens.push(Tokenizer::unraw_token(token));
                                    }
                                }
                            }
                            "popfront" => {
                                match &tokens[token_idx - 1].token { // get and replace previous token
                                    Token::Symbol('.') => {
                                        // popfront
                                        out_tokens.truncate(out_tokens.len() - 1);
                                        out_tokens.push(WrappedToken::from_with_line(Token::PopFrontAccess, token.src_line));
                                    }
                                    _ => {
                                        // previous token was not '.' access, popfront is a variable here
                                        out_tokens.push(Tokenizer::unraw_token(token));
                                    }
                                }
                            }
                            _ => {
                                out_tokens.push(Tokenizer::unraw_token(token));
                            }
                        }
                    }
                    _ => {
                        out_tokens.push(Tokenizer::unraw_token(token));
                    }
                }
            }
        }

        // validate scope closures, not wholly necessary here but for now it helps
        let mut scope_open_idxs: Vec<usize> = vec![];

        for (token_idx, token) in out_tokens.iter().enumerate() {
            match token.token {
                Token::ScopeOpen => {
                    scope_open_idxs.push(token_idx);
                }
                Token::ScopeClose => {
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

    fn consume_integer(&mut self) -> WrappedToken {
        let mut digit_str = String::new();

        while self.char_idx_in_bounds() && self.get_current_char().is_ascii_digit() {
            digit_str.push(self.get_current_char());
            self.char_idx += 1
        }

        WrappedToken::from_with_line(Token::IntegerLiteral(u32::from_str_radix(&digit_str, 10).unwrap()), self.line_idx + 1)
    }

    fn consume_identifier(&mut self) -> WrappedToken {
        let mut identifier_str = String::new();

        // identifiers may contain a number, only the start needs to be a letter
        while self.char_idx_in_bounds() && (self.get_current_char().is_ascii_alphanumeric() || self.get_current_char() == '_') {
            identifier_str.push(self.get_current_char());
            self.char_idx += 1
        }

        WrappedToken::from_with_line(Token::RawIdentifier(identifier_str), self.line_idx + 1)
    }

    fn consume_string_literal(&mut self) -> WrappedToken {
        let mut literal_str = String::new();
        self.char_idx += 1; // go into bounds of string

        while self.char_idx_in_bounds() && self.get_current_char() != '"' {
            literal_str.push(self.get_current_char());
            self.char_idx += 1
        }

        self.char_idx += 1; // leave string bounds
        WrappedToken::from_with_line(Token::StringLiteral(literal_str), self.line_idx + 1)
    }

    fn consume_whitespace(&mut self) -> WrappedToken {
        while self.char_idx_in_bounds() && self.get_current_char().is_ascii_whitespace() {
            self.char_idx += 1
        }

        WrappedToken::from_with_line(Token::Whitespace, self.line_idx + 1)
    }

    fn consume_comment(&mut self) -> WrappedToken {
        while self.char_idx_in_bounds() {
            self.char_idx += 1
        }

        WrappedToken::from(Token::Comment)
    }
}
