use std::ops::DerefMut;

use crate::tokenizer::Token;

#[derive(Debug)]
pub enum Expression {
    IntegerLiteral {
        value: u32,
    },
    StringLiteral {
        value: String,
    },
    Variable {
        name: String,
    },
}

#[derive(Debug)]
pub enum Statement {
    Return,
    Block,
    Assign,
    If,
    Equals,
    NotEquals,
    MoreThan,
    LessThan,
    Add,
    Sub,
}

#[derive(Debug)]
pub struct ASTToken {
    t_type: Option<Statement>,
    // args for arithmetic
    arg1: Option<Expression>,
    arg2: Option<Expression>,
    // args for nested code blocks (if/else)
    body: Vec<ASTToken>,
    else_body: Vec<ASTToken>,
}

impl ASTToken {
    pub fn empty() -> Self {
        Self {
            t_type: None, arg1: None, arg2: None, body: vec![], else_body: vec![]
        }
    }
    pub fn with_args(t_type: Statement, arg1: Expression, arg2: Expression) -> Self {
        Self {
            t_type: Some(t_type), arg1: Some(arg1), arg2: Some(arg2), body: vec![], else_body: vec![]
        }
    }
    pub fn push_body(&mut self, new_ast_token: ASTToken) {
        self.body.push(new_ast_token);
    }
}

pub struct ASTGenerator {
    tokens: Vec<Token>,
    current_token_idx: usize,
}

impl ASTGenerator {
    pub fn init(tokens: Vec<Token>) -> Self {
        Self { tokens, current_token_idx: 0 }
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
    fn resolve_variable_like_token(token: Token) -> Expression {
        match token {
            Token::IntegerLiteral { value } => {
                Expression::IntegerLiteral { value: value.to_owned() }
            }
            Token::StringLiteral { value } => {
                Expression::StringLiteral { value: value.to_owned() }
            }
            Token::Variable { name } => {
                Expression::Variable { name }
            }
            _ => {
                panic!("{:?} passed as value for variable-like token!", token)
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

    pub fn generate_ast(&mut self) -> Vec<ASTToken> {
        let mut generated_ast: Vec<ASTToken> = vec![];

        // root scope
        generated_ast.push(ASTToken::empty());
        //let mut current_scope = generated_ast.last_mut().unwrap();
        let mut scope_stack = vec![generated_ast.last_mut().unwrap()];
        let mut current_scope = scope_stack.last_mut().unwrap().deref_mut();

        while self.current_token_idx < self.tokens.len() {
            let current_token = self.get_token().to_owned();

            match current_token {
                Token::ScopeOpen => {

                }
                Token::ScopeClose => {
                    current_scope = scope_stack.pop().unwrap();
                }
                Token::If => {
                    // looking for any Variable/Literal, any comparison token and any Variable/Literal
                    let first_value_expression: Expression = ASTGenerator::resolve_variable_like_token(
                        self.advance_and_get_token().to_owned()
                    );
                    let comparison_statement: Statement = ASTGenerator::resolve_comparison_like_token(
                        self.advance_and_get_token().to_owned()
                    );
                    let second_value_expression: Expression = ASTGenerator::resolve_variable_like_token(
                        self.advance_and_get_token().to_owned()
                    );
                    let new_ast_token: ASTToken = ASTToken::with_args(
                        comparison_statement,
                        first_value_expression,
                        second_value_expression
                    );

                    current_scope.push_body(new_ast_token);
                    // check for block to execute after if statement
                    assert_eq!(*self.peek_next_token().unwrap(), Token::ScopeOpen);
                    current_scope = current_scope.body.last_mut().unwrap();
                    self.advance_token(); // skip scope open
                }
                Token::Alloc => {
                    // looking for Variable, Assign and any Literal or another Variable
                    let new_variable_expression: Expression;
                    if let Token::Variable { name } = self.advance_and_get_token() {
                        new_variable_expression = Expression::Variable { name: name.to_string() };
                    } else {
                        panic!("{:?} passed as Variable to Alloc!", current_token)
                    }

                    // just make sure the = is there
                    if let Token::Assign = self.advance_and_get_token() {} else {
                        panic!("{:?} passed as Assign to Alloc!", current_token)
                    }

                    let new_value_expression: Expression = ASTGenerator::resolve_variable_like_token(
                        self.advance_and_get_token().to_owned()
                    );

                    current_scope.push_body(
                        ASTToken::with_args(
                            Statement::Assign,
                            new_variable_expression,
                            new_value_expression
                        )
                    );

                    // check for line end, alloc takes a fixed amount of args
                    assert_eq!(*self.peek_next_token().unwrap(), Token::LineEnd);
                }
                _ => {

                }
            }

            self.advance_token();
        }

        generated_ast
    }
}
