use std::collections::HashMap;

use crate::tokenizer::{Token, WrappedToken};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    IntegerLiteral(u32),
    StringLiteral(String),
    BoolLiteral(bool),
    Variable(String),
    Array(Vec<Value>),
    Return,
    Null,
    Expression {
        values: Vec<Value>,
        operators: Vec<Operator>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Add,
    Sub,
    Equals,
    NotEquals,
    MoreThan,
    LessThan,
    MoreThanOrEquals,
    LessThanOrEquals,
    ArrayAccess,
    LenAccess,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Empty,
    Block,
    BlockEnd,
    Alloc,
    Set,
    DebugPrintCall,
    ReadLineCall,
    EOF,
    // conditions
    If(Operator),
    While(Operator),
    // jumps
    Jump(Option<usize>),
    Label(String),
    // subroutines
    SubroutineCall(Option<usize>),
    SubroutineReturn,
    SubroutineDefine,
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
    pub src_line: usize,
}

impl ASTToken {
    pub fn empty(src_line: usize) -> Self {
        Self {
            t_type: Statement::Empty, arg1: None, arg2: None, body_idx: None, body_extent: None, else_body_idx: None, src_line
        }
    }
    pub fn of_type(t_type: Statement, src_line: usize) -> Self {
        Self {
            t_type, arg1: None, arg2: None, body_idx: None, body_extent: None, else_body_idx: None, src_line
        }
    }
    pub fn with_args(t_type: Statement, arg1: Value, arg2: Option<Value>, src_line: usize) -> Self {
        Self {
            t_type, arg1: Some(arg1), arg2: arg2, body_idx: None, body_extent: None, else_body_idx: None, src_line
        }
    }
    pub fn with_args_and_body(t_type: Statement, arg1: Value, arg2: Option<Value>, body_idx: usize, else_body_idx: Option<usize>, src_line: usize) -> Self {
        Self {
            t_type, arg1: Some(arg1), arg2: arg2, body_idx: Some(body_idx), body_extent: None, else_body_idx, src_line
        }
    }
    pub fn new_scope(body_idx: usize, src_line: usize) -> Self {
        Self {
            t_type: Statement::Block, arg1: None, arg2: None, body_idx: Some(body_idx), body_extent: None, else_body_idx: None, src_line
        }
    }
}

pub struct ASTGenerator {
    tokens: Vec<WrappedToken>,
    current_token_idx: usize,
    pub generated_ast: Vec<ASTToken>,
    scope_open_idxs: Vec<usize>,
    // label name, label index
    jump_table: HashMap<String, usize>,
    // label name to jump to, vec of indexes of orphaned jump
    jumps: HashMap<String, Vec<usize>>,
    // subroutine name, subroutine index
    subroutine_table: HashMap<String, usize>,
    // subroutine name to call, vec of indexes of calls
    subroutine_calls: HashMap<String, Vec<usize>>,
}

impl ASTGenerator {
    pub fn init(tokens: Vec<WrappedToken>) -> Self {
        Self {
            tokens,
            current_token_idx: 0,
            generated_ast: vec![],
            scope_open_idxs: vec![],
            jump_table: HashMap::new(),
            jumps: HashMap::new(),
            subroutine_table: HashMap::new(),
            subroutine_calls: HashMap::new(),
        }
    }
    fn advance_and_get_token(&mut self) -> &WrappedToken {
        self.current_token_idx += 1;
        &self.tokens[self.current_token_idx]
    }
    fn advance_token(&mut self) {
        self.current_token_idx += 1;
    }
    fn get_token(&mut self) -> &WrappedToken {
        &self.tokens[self.current_token_idx]
    }
    fn peek_next_token(&self) -> Option<&WrappedToken> {
        if self.current_token_idx < self.tokens.len() {
            Some(&self.tokens[self.current_token_idx + 1])
        } else {
            None
        }
    }
    fn resolve_value_from_token(token: &WrappedToken) -> Value {
        match &token.token {
            Token::IntegerLiteral(value) => Value::IntegerLiteral(value.to_owned()),
            Token::StringLiteral(value) => Value::StringLiteral(value.to_owned()),
            Token::BoolTrue => Value::BoolLiteral(true),
            Token::BoolFalse => Value::BoolLiteral(false),
            Token::Variable(value) => Value::Variable(value.to_owned()),
            _ => panic!("{:?} passed as value for variable read token!", token),
        }
    }
    fn resolve_variable_write_like_token(token: &WrappedToken) -> Value {
        match &token.token {
            Token::Variable(value) => {
                Value::Variable(value.to_owned())
            }
            _ => {
                panic!("{:?} passed as value for variable write token!", token)
            }
        }
    }
    fn resolve_variable_name_like_token(token: &WrappedToken) -> Option<String> {
        match &token.token {
            Token::Variable(value) => {
                Some(value.to_owned())
            }
            _ => {
                panic!("{:?} passed as value for variable write token!", token)
            }
        }
    }
    fn resolve_comparison_like_token(token: &WrappedToken) -> Operator {
        match &token.token {
            Token::Equals => Operator::Equals,
            Token::NotEquals => Operator::NotEquals,
            Token::MoreThan => Operator::MoreThan,
            Token::LessThan => Operator::LessThan,
            Token::MoreThanOrEquals => Operator::MoreThanOrEquals,
            Token::LessThanOrEquals => Operator::LessThanOrEquals,
            _ => panic!("{:?} passed as value for comparison-like token!", token),
        }
    }
    fn advance_and_gather_tokens_for_value(&mut self) -> Vec<WrappedToken> {
        let mut tokens: Vec<WrappedToken> = vec![];

        while
            !ASTGenerator::token_is_scope_like(self.peek_next_token().unwrap())
            && !ASTGenerator::token_is_line_end(self.peek_next_token().unwrap())
        {
            tokens.push(self.advance_and_get_token().to_owned());
        }

        tokens
    }
    fn resolve_any_value(tokens: Vec<WrappedToken>) -> Value {
        if tokens.len() == 1 {
            // single literal
            return ASTGenerator::resolve_value_from_token(tokens.get(0).unwrap());
        } else if tokens.len() > 1 {
            let mut array_scratch: Vec<Value> = vec![];
            let mut token_idx = 0;
            let mut value_tokens: Vec<Value> = vec![];
            let mut operator_tokens: Vec<Operator> = vec![];
    
            while token_idx < tokens.len() {
                if tokens[token_idx].token == Token::ArrayOpen {
                    // handle array
                    token_idx += 1;
    
                    while tokens[token_idx].token != Token::ArrayClose {
                        let this_token = tokens[token_idx].to_owned();
    
                        if ASTGenerator::token_is_assign_like(&this_token)
                        || ASTGenerator::token_is_assign_op_like(&this_token)
                        || ASTGenerator::token_is_comparison_like(&this_token)
                        || ASTGenerator::token_is_line_end(&this_token)
                        || ASTGenerator::token_is_scope_like(&this_token) {
                            panic!("Array incomplete!");
                        }

                        match tokens[token_idx].token {
                            Token::ParensOpen => {
                                // coalesce tokens in ()
                                let mut parens_tokens: Vec<WrappedToken> = vec![];
                                let mut parens_deep: usize = 0;
                                // skip opening parens
                                token_idx += 1;
    
                                while tokens[token_idx].token != Token::ParensClose || parens_deep > 0 {
                                    if tokens[token_idx].token == Token::ParensOpen {
                                        parens_deep += 1;
                                    } else if tokens[token_idx].token == Token::ParensClose {
                                        parens_deep -= 1;
                                    }
    
                                    parens_tokens.push(tokens[token_idx].to_owned());
                                    token_idx += 1;
                                }
    
                                array_scratch.push(ASTGenerator::resolve_any_value(parens_tokens));
                            }
                            _ => {
                                array_scratch.push(ASTGenerator::resolve_value_from_token(&this_token));
                            }
                        }
    
                        token_idx += 1;
                    }
    
                    value_tokens.push(Value::Array(array_scratch.to_owned()));
                    array_scratch.clear();
                } else if ASTGenerator::token_is_comparison_like(&tokens[token_idx]) {
                    // add new operator and move temp tokens to list of token lists
                    operator_tokens.push(ASTGenerator::resolve_comparison_like_token(&tokens[token_idx]));
                } else {
                    let this_token = tokens[token_idx].to_owned();
    
                    match &this_token.token {
                        // values
                        Token::IntegerLiteral(_) => value_tokens.push(ASTGenerator::resolve_value_from_token(&this_token)),
                        Token::StringLiteral(_) => value_tokens.push(ASTGenerator::resolve_value_from_token(&this_token)),
                        Token::BoolTrue => value_tokens.push(ASTGenerator::resolve_value_from_token(&this_token)),
                        Token::BoolFalse => value_tokens.push(ASTGenerator::resolve_value_from_token(&this_token)),
                        Token::Variable(_) => value_tokens.push(ASTGenerator::resolve_value_from_token(&this_token)),
                        // operators
                        Token::Plus => operator_tokens.push(Operator::Add),
                        Token::Minus => operator_tokens.push(Operator::Sub),
                        Token::ArrayAccess => {
                            // accessing array from previous value token, coalesce
                            let array_value = value_tokens.pop().unwrap();
                            let mut access_tokens: Vec<WrappedToken> = vec![];
                            // skip opening array access
                            token_idx += 1;

                            while tokens[token_idx].token != Token::ArrayAccess {
                                access_tokens.push(tokens[token_idx].to_owned());
                                token_idx += 1;
                            }

                            value_tokens.push(
                                Value::Expression {
                                    values: vec![
                                        array_value,
                                        ASTGenerator::resolve_any_value(access_tokens)
                                    ],
                                    operators: vec![Operator::ArrayAccess],
                                }
                            );
                        },
                        Token::LenAccess => {
                            // accessing length of previous value token, coalesce
                            let value = value_tokens.pop().unwrap();

                            value_tokens.push(
                                Value::Expression {
                                    values: vec![
                                        value,
                                        Value::Null
                                    ],
                                    operators: vec![Operator::LenAccess],
                                }
                            );
                        },
                        Token::ParensOpen => {
                            // coalesce tokens in ()
                            let mut parens_tokens: Vec<WrappedToken> = vec![];
                            let mut parens_deep: usize = 0;
                            // skip opening parens
                            token_idx += 1;

                            while tokens[token_idx].token != Token::ParensClose || parens_deep > 0 {
                                if tokens[token_idx].token == Token::ParensOpen {
                                    parens_deep += 1;
                                } else if tokens[token_idx].token == Token::ParensClose {
                                    parens_deep -= 1;
                                }

                                parens_tokens.push(tokens[token_idx].to_owned());
                                token_idx += 1;
                            }

                            value_tokens.push(ASTGenerator::resolve_any_value(parens_tokens));
                        }
                        _ => panic!("{:?} passed as value for variable read token!", this_token),
                    }
                }
    
                token_idx += 1;
            }

            return Value::Expression {
                values: value_tokens,
                operators: operator_tokens
            };
        } else {
            panic!("Invalid operand length!");
        }
    }
    fn unpack_expression(expression: &Value) -> (Vec<Value>, Vec<Operator>) {
        match expression {
            Value::Expression { values, operators } => {
                return (values.to_owned(), operators.to_owned());
            }
            _ => {
                return (vec![expression.to_owned()], vec![]);
            }
        }
    }
    fn token_is_comparison_like(token: &WrappedToken) -> bool {
        match token.token {
            Token::Equals |
            Token::NotEquals |
            Token::MoreThan |
            Token::LessThan => true,
            _ => false,
        }
    }
    fn token_is_assign_op_like(token: &WrappedToken) -> bool {
        match token.token {
            Token::PlusEquals |
            Token::MinusEquals => true,
            _ => false,
        }
    }
    fn token_is_assign_like(token: &WrappedToken) -> bool {
        match token.token {
            Token::Assign => true,
            _ => false,
        }
    }
    fn token_is_scope_like(token: &WrappedToken) -> bool {
        match token.token {
            Token::ScopeOpen => true,
            _ => false,
        }
    }
    fn token_is_line_end(token: &WrappedToken) -> bool {
        match token.token {
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
    fn insert_new_empty_ast_scope(&mut self, src_line: usize) {
        self.insert_new_ast_scope(
            ASTToken::new_scope(
                self.generated_ast.len() + 1, // point to index after scope open
                src_line,
            )
        );
    }
    fn insert_ast_token_at_end(&mut self, new_token: ASTToken) {
        self.generated_ast.push(new_token);
    }
    fn insert_label(&mut self, label_name: String) {
        self.insert_ast_token_at_end(ASTToken::of_type(
            Statement::Label(label_name.to_owned()),
            0,
        ));
        self.jump_table.insert(label_name.to_owned(), self.generated_ast.len() - 1);
    }
    fn insert_dummy_jump(&mut self, jump_name: String, src_line: usize) {
        self.insert_ast_token_at_end(ASTToken::of_type(
            Statement::Jump(None),
            src_line,
        ));
        self.jumps.entry(
            jump_name
        ).or_insert_with(
            Vec::new
        ).push(
            self.generated_ast.len() - 1
        );
    }
    fn insert_subroutine(&mut self, subroutine_name: String) {
        self.insert_ast_token_at_end(ASTToken::of_type(
            Statement::SubroutineDefine,
            0,
        ));
        // index after definition, so the interpreter doesn't skip
        self.subroutine_table.insert(subroutine_name.to_owned(), self.generated_ast.len());
    }
    fn insert_subroutine_call(&mut self, subroutine_name: String, src_line: usize) {
        self.insert_ast_token_at_end(ASTToken::of_type(
            Statement::SubroutineCall(None),
            src_line,
        ));
        self.subroutine_calls.entry(
            subroutine_name
        ).or_insert_with(
            Vec::new
        ).push(
            self.generated_ast.len() - 1
        );
    }
    pub fn generate_ast(&mut self) {
        self.insert_root_ast_scope(ASTToken::empty(0)); // root scope

        while self.current_token_idx < self.tokens.len() {
            let current_token = self.get_token().to_owned();

            match &current_token.token {
                Token::ScopeOpen => {
                    self.insert_new_empty_ast_scope(current_token.src_line);
                }
                Token::ScopeClose => {
                    let closing_scope_idx = self.scope_open_idxs.pop().unwrap();

                    if self.generated_ast[closing_scope_idx - 1].t_type == Statement::SubroutineDefine {
                        // this is closing a function call, ensure the last token is return
                        if self.generated_ast[self.generated_ast.len() - 1].t_type != Statement::SubroutineReturn {
                            // just return false
                            self.insert_ast_token_at_end(ASTToken::with_args(
                                Statement::SubroutineReturn,
                                Value::BoolLiteral(false),
                                None,
                                0,
                            ));
                        }
                    }
                    self.generated_ast[closing_scope_idx].body_extent = Some(
                        self.generated_ast.len() - closing_scope_idx
                    );
                    self.insert_ast_token_at_end(ASTToken::of_type(Statement::BlockEnd, current_token.src_line));
                }
                Token::EOF => {
                    self.scope_open_idxs.pop();
                    self.insert_ast_token_at_end(ASTToken::of_type(Statement::EOF, 0));
                }
                Token::Label => {
                    // create new label with name
                    let label_name = ASTGenerator::resolve_variable_name_like_token(
                        self.advance_and_get_token()
                    ).expect(&format!("Label name not passed to label on line {}!", current_token.src_line));

                    self.insert_label(label_name);
                    // check for line end
                    assert_eq!(self.peek_next_token().unwrap().token, Token::LineEnd);
                }
                Token::Jump => {
                    eprintln!("{}Warning: JUMPING IS UNSAFE!{}", "\x1b[38;5;214m", "\x1b[0m");
                    // create new jump
                    let label_name = ASTGenerator::resolve_variable_name_like_token(
                        self.advance_and_get_token()
                    ).expect(&format!("Label name not passed to jump on line {}!", current_token.src_line));

                    self.insert_dummy_jump(label_name, current_token.src_line);
                    // check for line end
                    assert_eq!(self.peek_next_token().unwrap().token, Token::LineEnd);
                }
                Token::SubroutineCall => {
                    let subroutine_name = ASTGenerator::resolve_variable_name_like_token(
                        self.advance_and_get_token()
                    ).unwrap();

                    if self.peek_next_token().unwrap_or(
                        &WrappedToken::from(Token::LineEnd)
                    ).token == Token::LineEnd {
                        // line end after sub name, just insert sub call
                        self.insert_subroutine_call(
                            subroutine_name, current_token.src_line
                        );
                    } else {
                        // check for -> and variable name to assign return to
                        if self.advance_and_get_token().token != Token::SubroutineDirect {
                            panic!("{:?} passed as redirect to SubroutineCall on line {}!", current_token, current_token.src_line);
                        }
                        self.insert_subroutine_call(
                            subroutine_name, current_token.src_line
                        );
                        // get the variable to assign to
                        let variable_expression: Value = ASTGenerator::resolve_variable_write_like_token(
                            self.advance_and_get_token()
                        );
                        // assign the special Return token to the variable
                        let new_token = ASTToken::with_args(
                            Statement::Set,
                            variable_expression,
                            Some(Value::Return),
                            current_token.src_line,
                        );
                        self.insert_ast_token_at_end(new_token);
                    }

                    assert_eq!(self.peek_next_token().unwrap().token, Token::LineEnd);
                }
                Token::SubroutineReturn => {
                    let new_token: ASTToken;

                    if self.peek_next_token().unwrap().token != Token::LineEnd {
                        let value_token = ASTGenerator::resolve_any_value(self.advance_and_gather_tokens_for_value());
                        let (values, operators) = ASTGenerator::unpack_expression(&value_token);

                        if operators.len() == 0 {
                            new_token = ASTToken::with_args(
                                Statement::SubroutineReturn,
                                values.get(0).unwrap().to_owned(),
                                None,
                                current_token.src_line,
                            );
                        } else {
                            new_token = ASTToken::with_args(
                                Statement::SubroutineReturn,
                                Value::Expression { values: values, operators: operators },
                                None,
                                current_token.src_line,
                            );
                        }
                    } else {
                        // return false if no value was passed to ret
                        new_token = ASTToken::with_args(
                            Statement::SubroutineReturn,
                            Value::BoolLiteral(false),
                            None,
                            current_token.src_line,
                        );
                    }

                    self.insert_ast_token_at_end(new_token);
                    assert_eq!(self.peek_next_token().unwrap().token, Token::LineEnd);
                }
                Token::SubroutineDefine => {
                    // name of new subroutine
                    let subroutine_name = ASTGenerator::resolve_variable_name_like_token(
                        self.advance_and_get_token()
                    ).unwrap();
                    // add subroutine token to stack
                    self.insert_subroutine(subroutine_name);
                    // check for block to execute after if statement
                    assert_eq!(self.peek_next_token().unwrap().token, Token::ScopeOpen);
                    self.insert_new_empty_ast_scope(current_token.src_line);
                    self.advance_token(); // skip scope open
                }
                Token::If => {
                    let value_token = ASTGenerator::resolve_any_value(self.advance_and_gather_tokens_for_value());
                    let (values, operators) = ASTGenerator::unpack_expression(&value_token);
                    let new_token: ASTToken;

                    if operators.len() == 0 {
                        // implicit bool
                        new_token = ASTToken::with_args_and_body(
                            Statement::If(Operator::Equals),
                            values[0].to_owned(),
                            Some(Value::BoolLiteral(true)),
                            self.generated_ast.len() + 1,
                            None,
                            current_token.src_line,
                        );
                    } else {
                        new_token = ASTToken::with_args_and_body(
                            Statement::If(operators[0].to_owned()),
                            values[0].to_owned(),
                            Some(values[1].to_owned()),
                            self.generated_ast.len() + 1,
                            None,
                            current_token.src_line,
                        );
                    }

                    // add new token to stack
                    self.insert_ast_token_at_end(new_token);
                    // check for block to execute after if statement
                    assert_eq!(self.peek_next_token().unwrap().token, Token::ScopeOpen);
                    self.insert_new_empty_ast_scope(current_token.src_line);
                    self.advance_token(); // skip scope open
                }
                Token::While => {
                    let value_token = ASTGenerator::resolve_any_value(self.advance_and_gather_tokens_for_value());
                    let (values, operators) = ASTGenerator::unpack_expression(&value_token);
                    let new_token: ASTToken;

                    if operators.len() == 0 {
                        // implicit bool
                        new_token = ASTToken::with_args_and_body(
                            Statement::While(Operator::Equals),
                            values[0].to_owned(),
                            Some(Value::BoolLiteral(true)),
                            self.generated_ast.len() + 1,
                            None,
                            current_token.src_line,
                        );
                    } else {
                        new_token = ASTToken::with_args_and_body(
                            Statement::While(operators[0].to_owned()),
                            values[0].to_owned(),
                            Some(values[1].to_owned()),
                            self.generated_ast.len() + 1,
                            None,
                            current_token.src_line,
                        );
                    }

                    // add new token to stack
                    self.insert_ast_token_at_end(new_token);
                    // check for block to execute after if statement
                    assert_eq!(self.peek_next_token().unwrap().token, Token::ScopeOpen);
                    self.insert_new_empty_ast_scope(current_token.src_line);
                    self.advance_token(); // skip scope open
                }
                Token::Alloc => {
                    // get the variable to assign to
                    let variable_expression: Value = ASTGenerator::resolve_variable_write_like_token(
                        self.advance_and_get_token()
                    );

                    // make sure the = is there
                    if !ASTGenerator::token_is_assign_like(self.advance_and_get_token()) {
                        panic!("{:?} passed as Assign to Alloc on line {}!", current_token, current_token.src_line);
                    }

                    let value_token = ASTGenerator::resolve_any_value(self.advance_and_gather_tokens_for_value());
                    let (values, operators) = ASTGenerator::unpack_expression(&value_token);
                    let new_token: ASTToken;

                    if operators.len() == 0 {
                        new_token = ASTToken::with_args(
                            Statement::Alloc,
                            variable_expression,
                            Some(values.get(0).unwrap().to_owned()),
                            current_token.src_line,
                        );
                    } else {
                        new_token = ASTToken::with_args(
                            Statement::Alloc,
                            variable_expression,
                            Some(Value::Expression { values: values, operators: operators }),
                            current_token.src_line,
                        );
                    }

                    self.insert_ast_token_at_end(new_token);
                    // check for line end, alloc takes a fixed amount of args
                    assert_eq!(self.peek_next_token().unwrap().token, Token::LineEnd);
                }
                Token::Set => {
                    // get the variable to assign to
                    let variable_expression: Value = ASTGenerator::resolve_variable_write_like_token(
                        self.advance_and_get_token()
                    );

                    // make sure the = is there
                    if !ASTGenerator::token_is_assign_like(self.advance_and_get_token()) {
                        panic!("{:?} passed as Assign to Set on line {}!", current_token, current_token.src_line);
                    }

                    let value_token = ASTGenerator::resolve_any_value(self.advance_and_gather_tokens_for_value());
                    let (values, operators) = ASTGenerator::unpack_expression(&value_token);
                    let new_token: ASTToken;

                    if operators.len() == 0 {
                        new_token = ASTToken::with_args(
                            Statement::Set,
                            variable_expression,
                            Some(values.get(0).unwrap().to_owned()),
                            current_token.src_line,
                        );
                    } else {
                        new_token = ASTToken::with_args(
                            Statement::Set,
                            variable_expression,
                            Some(Value::Expression { values: values, operators: operators }),
                            current_token.src_line,
                        );
                    }

                    self.insert_ast_token_at_end(new_token);
                    // check for line end, set takes a fixed amount of args
                    assert_eq!(self.peek_next_token().unwrap().token, Token::LineEnd);
                }
                Token::Variable(_) => {
                    let new_token: ASTToken;

                    if ASTGenerator::token_is_assign_op_like(self.peek_next_token().unwrap()) {
                        // plus and minus equals operators
                        let variable_expression: Value = ASTGenerator::resolve_variable_write_like_token(
                            &current_token
                        );
                        let assign_op: WrappedToken;

                        match self.advance_and_get_token().to_owned().token {
                            Token::PlusEquals => {
                                assign_op = WrappedToken::from(Token::Plus);
                            }
                            Token::MinusEquals => {
                                assign_op = WrappedToken::from(Token::Minus);
                            }
                            _ => {
                                unreachable!()
                            }
                        }

                        let value_token = ASTGenerator::resolve_any_value(
                            [
                                vec![current_token.to_owned(), assign_op],
                                self.advance_and_gather_tokens_for_value(),
                            ].concat()
                        );

                        new_token = ASTToken::with_args(
                            Statement::Set,
                            variable_expression,
                            Some(value_token),
                            current_token.src_line,
                        );
                    } else {
                        panic!("Mysterious variable at start of statement with no assign operator on line {}!", current_token.src_line);
                    }

                    self.insert_ast_token_at_end(new_token);
                    // check for line end, alloc takes a fixed amount of args
                    assert_eq!(self.peek_next_token().unwrap().token, Token::LineEnd);
                }
                Token::Print => {
                    // debug printing, takes 1 variable-like argument
                    let value_token = ASTGenerator::resolve_any_value(self.advance_and_gather_tokens_for_value());
                    let (values, operators) = ASTGenerator::unpack_expression(&value_token);
                    let new_token: ASTToken;

                    if operators.len() == 0 {
                        new_token = ASTToken::with_args(
                            Statement::DebugPrintCall,
                            values.get(0).unwrap().to_owned(),
                            None,
                            current_token.src_line,
                        );
                    } else {
                        new_token = ASTToken::with_args(
                            Statement::DebugPrintCall,
                            Value::Expression { values: values, operators: operators },
                            None,
                            current_token.src_line,
                        );
                    }

                    self.insert_ast_token_at_end(new_token);
                    assert_eq!(self.peek_next_token().unwrap().token, Token::LineEnd);
                }
                Token::ReadLine => {
                    // read line of input from terminal, takes 1 variable argument
                    let variable_expression: Value = ASTGenerator::resolve_variable_write_like_token(
                        self.advance_and_get_token()
                    );
                    let new_token: ASTToken = ASTToken::with_args(
                        Statement::ReadLineCall,
                        variable_expression,
                        None,
                        current_token.src_line,
                    );
                    self.insert_ast_token_at_end(new_token);
                    assert_eq!(self.peek_next_token().unwrap().token, Token::LineEnd);
                }
                _ => {

                }
            }

            self.advance_token();
        }

        // resolve jumps
        for (key, value) in &self.jumps {
            for jump_idx in value {
                self.generated_ast[*jump_idx] = ASTToken::of_type(
                    Statement::Jump(
                        Some(*self.jump_table.get(key).unwrap())
                    ),
                    0,
                )
            }
        }
        self.jumps.clear();

        // resolve subroutines
        for (key, value) in &self.subroutine_calls {
            for call_idx in value {
                self.generated_ast[*call_idx] = ASTToken::of_type(
                    Statement::SubroutineCall(
                        Some(*self.subroutine_table.get(key).unwrap())
                    ),
                    0,
                )
            }
        }
        self.subroutine_calls.clear();
    }
}
