use crate::tokenizer::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    IntegerLiteral(u32),
    StringLiteral(String),
    BoolLiteral(bool),
    Variable(String),
    Expression {
        values: Vec<Value>,
        operators: Vec<Operator>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Comparison {
    Equals,
    NotEquals,
    MoreThan,
    LessThan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Add,
    Sub,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Empty,
    Block,
    BlockEnd,
    Allocate,
    Set,
    DebugPrintCall,
    ReadLineCall,
    EOF,
    // conditions
    If(Comparison),
    While(Comparison),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ASTToken {
    pub t_type: Statement,
    // args for arithmetic
    pub arg1: Option<Value>,
    pub arg2: Option<Value>,
    // args for nested code blocks (if/else)
    pub body_idx: Option<usize>,
    pub body_extent: Option<usize>,
    pub else_body_idx: Option<usize>,
}

impl ASTToken {
    pub fn empty() -> Self {
        Self {
            t_type: Statement::Empty, arg1: None, arg2: None, body_idx: None, body_extent: None, else_body_idx: None
        }
    }
    pub fn of_type(t_type: Statement) -> Self {
        Self {
            t_type, arg1: None, arg2: None, body_idx: None, body_extent: None, else_body_idx: None
        }
    }
    pub fn with_args(t_type: Statement, arg1: Value, arg2: Option<Value>) -> Self {
        Self {
            t_type, arg1: Some(arg1), arg2: arg2, body_idx: None, body_extent: None, else_body_idx: None
        }
    }
    pub fn with_args_and_body(t_type: Statement, arg1: Value, arg2: Option<Value>, body_idx: usize, else_body_idx: Option<usize>) -> Self {
        Self {
            t_type, arg1: Some(arg1), arg2: arg2, body_idx: Some(body_idx), body_extent: None, else_body_idx
        }
    }
    pub fn new_scope(body_idx: usize) -> Self {
        Self {
            t_type: Statement::Block, arg1: None, arg2: None, body_idx: Some(body_idx), body_extent: None, else_body_idx: None
        }
    }
}

pub struct ASTGenerator {
    tokens: Vec<Token>,
    current_token_idx: usize,
    pub generated_ast: Vec<ASTToken>,
    scope_open_idxs: Vec<usize>,
}

impl ASTGenerator {
    pub fn init(tokens: Vec<Token>) -> Self {
        Self {
            tokens, current_token_idx: 0, generated_ast: vec![], scope_open_idxs: vec![]
        }
    }
    fn advance_and_get_token(&mut self) -> &Token {
        self.current_token_idx += 1;
        &self.tokens[self.current_token_idx]
    }
    fn advance_token(&mut self) {
        self.current_token_idx += 1;
    }
    fn get_token(&mut self) -> &Token {
        &self.tokens[self.current_token_idx]
    }
    fn peek_next_token(&self) -> Option<&Token> {
        if self.current_token_idx < self.tokens.len() {
            Some(&self.tokens[self.current_token_idx + 1])
        } else {
            None
        }
    }
    fn resolve_variable_read_like_token(token: Token) -> Value {
        match token {
            Token::IntegerLiteral { value } => Value::IntegerLiteral(value.to_owned()),
            Token::StringLiteral { value } => Value::StringLiteral(value.to_owned()),
            Token::BoolTrue => Value::BoolLiteral(true),
            Token::BoolFalse => Value::BoolLiteral(false),
            Token::Variable { name } => Value::Variable(name),
            _ => panic!("{:?} passed as value for variable read token!", token),
        }
    }
    fn resolve_value_from_token(token: Token) -> Value {
        match token {
            Token::IntegerLiteral { value } => Value::IntegerLiteral(value),
            Token::StringLiteral { value } => Value::StringLiteral(value),
            Token::BoolTrue => Value::BoolLiteral(true),
            Token::BoolFalse => Value::BoolLiteral(false),
            Token::Variable { name } => Value::Variable(name),
            _ => panic!("{:?} passed as value for variable read token!", token),
        }
    }
    fn resolve_variable_read_like_expression(tokens: Vec<Token>) -> Value {
        if tokens.len() < 2 {
            return ASTGenerator::resolve_value_from_token(tokens.last().unwrap().clone());
        }
    
        let mut value_tokens: Vec<Value> = vec![];
        let mut operator_tokens: Vec<Operator> = vec![];

        for token in tokens {
            match token {
                // values
                Token::IntegerLiteral { value: _ } => value_tokens.push(ASTGenerator::resolve_variable_read_like_token(token)),
                Token::StringLiteral { value: _ } => value_tokens.push(ASTGenerator::resolve_variable_read_like_token(token)),
                Token::BoolTrue => value_tokens.push(ASTGenerator::resolve_variable_read_like_token(token)),
                Token::BoolFalse => value_tokens.push(ASTGenerator::resolve_variable_read_like_token(token)),
                Token::Variable { name: _ } => value_tokens.push(ASTGenerator::resolve_variable_read_like_token(token)),
                // operators
                Token::Plus => operator_tokens.push(Operator::Add),
                Token::Minus => operator_tokens.push(Operator::Sub),
                _ => panic!("{:?} passed as value for variable read token!", token),
            }
        }

        assert_eq!(value_tokens.len() - 1, operator_tokens.len());
        return Value::Expression {
            values: value_tokens,
            operators: operator_tokens
        };
    }
    fn resolve_variable_write_like_token(token: Token) -> Value {
        match token {
            Token::Variable { name } => {
                Value::Variable(name)
            }
            _ => {
                panic!("{:?} passed as value for variable write token!", token)
            }
        }
    }
    fn resolve_comparison_like_token(token: Token) -> Comparison {
        match token {
            Token::Equals => Comparison::Equals,
            Token::NotEquals => Comparison::NotEquals,
            Token::MoreThan => Comparison::MoreThan,
            Token::LessThan => Comparison::LessThan,
            _ => panic!("{:?} passed as value for comparison-like token!", token),
        }
    }
    fn token_is_comparison_like(token: Token) -> bool {
        match token {
            Token::Equals |
            Token::NotEquals |
            Token::MoreThan |
            Token::LessThan => true,
            _ => false,
        }
    }
    fn token_is_scope_like(token: Token) -> bool {
        match token {
            Token::ScopeOpen => true,
            _ => false,
        }
    }
    fn token_is_line_end(token: Token) -> bool {
        match token {
            Token::LineEnd => true,
            _ => false,
        }
    }
    fn insert_root_ast_scope(&mut self, new_token: ASTToken) {
        self.generated_ast.push(new_token);
    }
    fn insert_new_ast_scope(&mut self, new_token: ASTToken) {
        self.generated_ast.push(new_token);
        self.scope_open_idxs.push(self.generated_ast.len() - 1); // new scope's index
    }
    fn insert_new_empty_ast_scope(&mut self) {
        self.insert_new_ast_scope(
            ASTToken::new_scope(
                self.generated_ast.len() + 1 // point to index after scope open
            )
        );
    }
    fn insert_ast_token_at_end(&mut self, new_token: ASTToken) {
        self.generated_ast.push(new_token);
    }

    pub fn generate_ast(&mut self) {
        self.insert_root_ast_scope(ASTToken::empty()); // root scope

        while self.current_token_idx < self.tokens.len() {
            let current_token = self.get_token().to_owned();
    
            match current_token {
                Token::ScopeOpen => {
                    self.insert_new_empty_ast_scope();
                }
                Token::ScopeClose => {
                    let closing_scope_idx = self.scope_open_idxs.pop().unwrap();
                    self.generated_ast[closing_scope_idx].body_extent = Some(
                        self.generated_ast.len() - closing_scope_idx
                    );
                    self.insert_ast_token_at_end(ASTToken::of_type(Statement::BlockEnd));
                }
                Token::EOF => {
                    self.scope_open_idxs.pop();
                    self.insert_ast_token_at_end(ASTToken::of_type(Statement::EOF));
                }
                Token::If => {
                    // first half
                    let mut first_value_tokens: Vec<Token> = vec![];
                    while !ASTGenerator::token_is_comparison_like(self.peek_next_token().unwrap().to_owned()) {
                        first_value_tokens.push(self.advance_and_get_token().to_owned());
                    }
                    let first_value_expression: Value = ASTGenerator::resolve_variable_read_like_expression(
                        first_value_tokens
                    );

                    // comparison
                    let comparison_operator: Comparison = ASTGenerator::resolve_comparison_like_token(
                        self.advance_and_get_token().to_owned()
                    );

                    // second half
                    let mut second_value_tokens: Vec<Token> = vec![];
                    while !ASTGenerator::token_is_scope_like(self.peek_next_token().unwrap().to_owned()) {
                        second_value_tokens.push(self.advance_and_get_token().to_owned());
                    }
                    let second_value_expression: Value = ASTGenerator::resolve_variable_read_like_expression(
                        second_value_tokens
                    );

                    // build token
                    let new_token: ASTToken = ASTToken::with_args_and_body(
                        Statement::If(comparison_operator),
                        first_value_expression,
                        Some(second_value_expression),
                        self.generated_ast.len() + 1,
                        None,
                    );
                    // add new token to stack
                    self.insert_ast_token_at_end(new_token);
                    // check for block to execute after if statement
                    assert_eq!(*self.peek_next_token().unwrap(), Token::ScopeOpen);
                    self.insert_new_empty_ast_scope();
                    self.advance_token(); // skip scope open
                }
                Token::While => {
                    // first half
                    let mut first_value_tokens: Vec<Token> = vec![];
                    while !ASTGenerator::token_is_comparison_like(self.peek_next_token().unwrap().to_owned()) {
                        first_value_tokens.push(self.advance_and_get_token().to_owned());
                    }
                    let first_value_expression: Value = ASTGenerator::resolve_variable_read_like_expression(
                        first_value_tokens
                    );

                    // comparison
                    let comparison_operator: Comparison = ASTGenerator::resolve_comparison_like_token(
                        self.advance_and_get_token().to_owned()
                    );

                    // second half
                    let mut second_value_tokens: Vec<Token> = vec![];
                    while !ASTGenerator::token_is_scope_like(self.peek_next_token().unwrap().to_owned()) {
                        second_value_tokens.push(self.advance_and_get_token().to_owned());
                    }
                    let second_value_expression: Value = ASTGenerator::resolve_variable_read_like_expression(
                        second_value_tokens
                    );

                    // build token
                    let new_token: ASTToken = ASTToken::with_args_and_body(
                        Statement::While(comparison_operator),
                        first_value_expression,
                        Some(second_value_expression),
                        self.generated_ast.len() + 1,
                        None,
                    );
                    // add new token to stack
                    self.insert_ast_token_at_end(new_token);
                    // check for block to execute after if statement
                    assert_eq!(*self.peek_next_token().unwrap(), Token::ScopeOpen);
                    self.insert_new_empty_ast_scope();
                    self.advance_token(); // skip scope open
                }
                Token::Alloc => {
                    // looking for Variable, Assign and any Literal or another Variable
                    let variable_expression: Value = ASTGenerator::resolve_variable_write_like_token(
                        self.advance_and_get_token().to_owned()
                    );
                    // just make sure the = is there
                    if let Token::Assign = self.advance_and_get_token() {} else {
                        panic!("{:?} passed as Assign to Alloc!", current_token)
                    }
                    let mut new_value_tokens: Vec<Token> = vec![];
                    while !ASTGenerator::token_is_line_end(self.peek_next_token().unwrap().to_owned()) {
                        new_value_tokens.push(self.advance_and_get_token().to_owned());
                    }
                    let new_value_expression: Value = ASTGenerator::resolve_variable_read_like_expression(
                        new_value_tokens
                    );

                    // build token
                    let new_token: ASTToken = ASTToken::with_args(
                        Statement::Allocate,
                        variable_expression,
                        Some(new_value_expression),
                    );
                    self.insert_ast_token_at_end(new_token);
                    // check for line end, alloc takes a fixed amount of args
                    assert_eq!(*self.peek_next_token().unwrap(), Token::LineEnd);
                }
                Token::Set => {
                    // looking for Variable, Assign and any Literal or another Variable
                    let variable_expression: Value = ASTGenerator::resolve_variable_write_like_token(
                        self.advance_and_get_token().to_owned()
                    );
                    // just make sure the = is there
                    if let Token::Assign = self.advance_and_get_token() {} else {
                        panic!("{:?} passed as Assign to Alloc!", current_token)
                    }
                    let mut new_value_tokens: Vec<Token> = vec![];
                    while !ASTGenerator::token_is_line_end(self.peek_next_token().unwrap().to_owned()) {
                        new_value_tokens.push(self.advance_and_get_token().to_owned());
                    }
                    let new_value_expression: Value = ASTGenerator::resolve_variable_read_like_expression(
                        new_value_tokens
                    );

                    // build token
                    let new_token: ASTToken = ASTToken::with_args(
                        Statement::Set,
                        variable_expression,
                        Some(new_value_expression),
                    );
                    self.insert_ast_token_at_end(new_token);
                    // check for line end, alloc takes a fixed amount of args
                    assert_eq!(*self.peek_next_token().unwrap(), Token::LineEnd);
                }
                Token::Print => {
                    // debug printing, takes 1 variable-like argument
                    let mut new_value_tokens: Vec<Token> = vec![];
                    while !ASTGenerator::token_is_line_end(self.peek_next_token().unwrap().to_owned()) {
                        new_value_tokens.push(self.advance_and_get_token().to_owned());
                    }
                    let new_value_expression: Value = ASTGenerator::resolve_variable_read_like_expression(
                        new_value_tokens
                    );

                    // build token
                    let new_token: ASTToken = ASTToken::with_args(
                        Statement::DebugPrintCall,
                        new_value_expression,
                        None
                    );
                    self.insert_ast_token_at_end(new_token);
                    assert_eq!(*self.peek_next_token().unwrap(), Token::LineEnd);
                }
                Token::ReadLine => {
                    // read line of input from terminal, takes 1 variable argument
                    let variable_expression: Value = ASTGenerator::resolve_variable_write_like_token(
                        self.advance_and_get_token().to_owned()
                    );
                    let new_token: ASTToken = ASTToken::with_args(
                        Statement::ReadLineCall,
                        variable_expression,
                        None
                    );
                    self.insert_ast_token_at_end(new_token);
                    assert_eq!(*self.peek_next_token().unwrap(), Token::LineEnd);
                }
                _ => {

                }
            }

            self.advance_token();
        }
    }
}
