use crate::tokenizer::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    IntegerLiteral(u32),
    StringLiteral(String),
    Bool(bool),
    Variable(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Empty,
    Return,
    Block,
    BlockEnd,
    Allocate,
    Set,
    Equals,
    NotEquals,
    MoreThan,
    LessThan,
    Add,
    Sub,
    DebugPrintCall,
    ReadLineCall,
    EOF,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ASTToken {
    pub t_type: Statement,
    // args for arithmetic
    pub arg1: Option<Expression>,
    pub arg2: Option<Expression>,
    // args for nested code blocks (if/else)
    pub body_idx: Option<usize>,
    pub body_extent: Option<usize>,
    pub else_body_idx: Option<usize>,
    pub recurring: bool,
}

impl ASTToken {
    pub fn empty() -> Self {
        Self {
            t_type: Statement::Empty, arg1: None, arg2: None, body_idx: None, body_extent: None, else_body_idx: None, recurring: false
        }
    }
    pub fn of_type(t_type: Statement) -> Self {
        Self {
            t_type, arg1: None, arg2: None, body_idx: None, body_extent: None, else_body_idx: None, recurring: false
        }
    }
    pub fn with_args(t_type: Statement, arg1: Expression, arg2: Option<Expression>) -> Self {
        Self {
            t_type, arg1: Some(arg1), arg2: arg2, body_idx: None, body_extent: None, else_body_idx: None, recurring: false
        }
    }
    pub fn with_args_and_body(t_type: Statement, arg1: Expression, arg2: Option<Expression>, body_idx: usize, else_body_idx: Option<usize>, recurring: bool) -> Self {
        Self {
            t_type, arg1: Some(arg1), arg2: arg2, body_idx: Some(body_idx), body_extent: None, else_body_idx, recurring
        }
    }
    pub fn new_scope(body_idx: usize) -> Self {
        Self {
            t_type: Statement::Block, arg1: None, arg2: None, body_idx: Some(body_idx), body_extent: None, else_body_idx: None, recurring: false
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
    fn peek_next_token(&mut self) -> Option<&Token> {
        if self.current_token_idx < self.tokens.len() {
            Some(&self.tokens[self.current_token_idx + 1])
        } else {
            None
        }
    }
    fn resolve_variable_read_like_token(token: Token) -> Expression {
        match token {
            Token::IntegerLiteral { value } => {
                Expression::IntegerLiteral(value.to_owned())
            }
            Token::StringLiteral { value } => {
                Expression::StringLiteral(value.to_owned())
            }
            Token::BoolTrue => {
                Expression::Bool(true)
            }
            Token::BoolFalse => {
                Expression::Bool(false)
            }
            Token::Variable { name } => {
                Expression::Variable(name)
            }
            _ => {
                panic!("{:?} passed as value for variable read token!", token)
            }
        }
    }
    fn resolve_variable_write_like_token(token: Token) -> Expression {
        match token {
            Token::Variable { name } => {
                Expression::Variable(name)
            }
            _ => {
                panic!("{:?} passed as value for variable write token!", token)
            }
        }
    }
    fn resolve_comparison_like_token(token: Token) -> Statement {
        match token {
            Token::MoreThan => {
                Statement::MoreThan
            }
            Token::LessThan => {
                Statement::LessThan
            }
            Token::Equals => {
                Statement::Equals
            }
            Token::NotEquals => {
                Statement::NotEquals
            }
            _ => {
                panic!("{:?} passed as value for comparison-like token!", token)
            }
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
                    // looking for any Variable/Literal, any comparison token and any Variable/Literal
                    let first_value_expression: Expression = ASTGenerator::resolve_variable_read_like_token(
                        self.advance_and_get_token().to_owned()
                    );
                    let comparison_statement: Statement = ASTGenerator::resolve_comparison_like_token(
                        self.advance_and_get_token().to_owned()
                    );
                    let second_value_expression: Expression = ASTGenerator::resolve_variable_read_like_token(
                        self.advance_and_get_token().to_owned()
                    );
                    let new_token: ASTToken = ASTToken::with_args_and_body(
                        comparison_statement,
                        first_value_expression,
                        Some(second_value_expression),
                        self.generated_ast.len() + 1,
                        None,
                        false,
                    );
                    // add new token to stack this doesn't fucking work
                    self.insert_ast_token_at_end(new_token);
                    // check for block to execute after if statement
                    assert_eq!(*self.peek_next_token().unwrap(), Token::ScopeOpen);
                    // switch the current token to the new scope, doesn't fucking work
                    self.insert_new_empty_ast_scope();
                    self.advance_token(); // skip scope open
                }
                Token::While => {
                    // looking for any Variable/Literal, any comparison token and any Variable/Literal
                    let first_value_expression: Expression = ASTGenerator::resolve_variable_read_like_token(
                        self.advance_and_get_token().to_owned()
                    );
                    let comparison_statement: Statement = ASTGenerator::resolve_comparison_like_token(
                        self.advance_and_get_token().to_owned()
                    );
                    let second_value_expression: Expression = ASTGenerator::resolve_variable_read_like_token(
                        self.advance_and_get_token().to_owned()
                    );
                    let new_token: ASTToken = ASTToken::with_args_and_body(
                        comparison_statement,
                        first_value_expression,
                        Some(second_value_expression),
                        self.generated_ast.len() + 1,
                        None,
                        true,
                    );
                    // add new token to stack this doesn't fucking work
                    self.insert_ast_token_at_end(new_token);
                    // check for block to execute after if statement
                    assert_eq!(*self.peek_next_token().unwrap(), Token::ScopeOpen);
                    // switch the current token to the new scope, doesn't fucking work
                    self.insert_new_empty_ast_scope();
                    self.advance_token(); // skip scope open
                }
                Token::Alloc => {
                    // looking for Variable, Assign and any Literal or another Variable
                    let variable_expression: Expression = ASTGenerator::resolve_variable_write_like_token(
                        self.advance_and_get_token().to_owned()
                    );
                    // just make sure the = is there
                    if let Token::Assign = self.advance_and_get_token() {} else {
                        panic!("{:?} passed as Assign to Alloc!", current_token)
                    }
                    let new_value_expression: Expression = ASTGenerator::resolve_variable_read_like_token(
                        self.advance_and_get_token().to_owned()
                    );
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
                    let variable_expression: Expression = ASTGenerator::resolve_variable_write_like_token(
                        self.advance_and_get_token().to_owned()
                    );
                    // just make sure the = is there
                    if let Token::Assign = self.advance_and_get_token() {} else {
                        panic!("{:?} passed as Assign to Set!", current_token)
                    }
                    let new_value_expression: Expression = ASTGenerator::resolve_variable_read_like_token(
                        self.advance_and_get_token().to_owned()
                    );
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
                    let value_expression: Expression = ASTGenerator::resolve_variable_read_like_token(
                        self.advance_and_get_token().to_owned()
                    );
                    let new_token: ASTToken = ASTToken::with_args(
                        Statement::DebugPrintCall,
                        value_expression,
                        None
                    );
                    self.insert_ast_token_at_end(new_token);
                    assert_eq!(*self.peek_next_token().unwrap(), Token::LineEnd);
                }
                Token::ReadLine => {
                    // read line of input from terminal, takes 1 variable argument
                    let variable_expression: Expression = ASTGenerator::resolve_variable_write_like_token(
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
