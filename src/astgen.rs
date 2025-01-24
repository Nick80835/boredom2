use std::collections::HashMap;

use crate::tokenizer::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    IntegerLiteral(u32),
    StringLiteral(String),
    BoolLiteral(bool),
    Variable(String),
    Array(Vec<Value>),
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
    // label name, label index
    jump_table: HashMap<String, usize>,
    // label name to jump to, vec of indexes of orphaned jump
    orphaned_jumps: HashMap<String, Vec<usize>>,
}

impl ASTGenerator {
    pub fn init(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current_token_idx: 0,
            generated_ast: vec![],
            scope_open_idxs: vec![],
            jump_table: HashMap::new(),
            orphaned_jumps: HashMap::new(),
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
    fn resolve_value_from_token(token: &Token) -> Value {
        match token {
            Token::IntegerLiteral { value } => Value::IntegerLiteral(value.to_owned()),
            Token::StringLiteral { value } => Value::StringLiteral(value.to_owned()),
            Token::BoolTrue => Value::BoolLiteral(true),
            Token::BoolFalse => Value::BoolLiteral(false),
            Token::Variable { name } => Value::Variable(name.to_owned()),
            _ => panic!("{:?} passed as value for variable read token!", token),
        }
    }
    fn resolve_token_read_like_expression(tokens: Vec<Token>) -> Value {
        if tokens.len() < 2 {
            return ASTGenerator::resolve_value_from_token(tokens.last().unwrap());
        }

        let mut value_tokens: Vec<Value> = vec![];
        let mut operator_tokens: Vec<Operator> = vec![];
        let mut array_scratch: Vec<Value> = vec![];
        let mut token_idx = 0;

        while token_idx < tokens.len() {
            if tokens[token_idx] == Token::ArrayOpen {
                // handle array
                token_idx += 1;

                while tokens[token_idx] != Token::ArrayClose {
                    let this_token = tokens[token_idx].to_owned();

                    if ASTGenerator::token_is_assign_like(&this_token)
                    || ASTGenerator::token_is_assign_op_like(&this_token)
                    || ASTGenerator::token_is_comparison_like(&this_token)
                    || ASTGenerator::token_is_line_end(&this_token)
                    || ASTGenerator::token_is_scope_like(&this_token) {
                        panic!("Array incomplete!");
                    }

                    array_scratch.push(ASTGenerator::resolve_value_from_token(&this_token));
                    token_idx += 1;
                }

                value_tokens.push(Value::Array(array_scratch.to_owned()));
                array_scratch.clear();
            } else {
                let this_token = tokens[token_idx].to_owned();

                match this_token {
                    // values
                    Token::IntegerLiteral { value: _ } => value_tokens.push(ASTGenerator::resolve_value_from_token(&this_token)),
                    Token::StringLiteral { value: _ } => value_tokens.push(ASTGenerator::resolve_value_from_token(&this_token)),
                    Token::BoolTrue => value_tokens.push(ASTGenerator::resolve_value_from_token(&this_token)),
                    Token::BoolFalse => value_tokens.push(ASTGenerator::resolve_value_from_token(&this_token)),
                    Token::Variable { name: _ } => value_tokens.push(ASTGenerator::resolve_value_from_token(&this_token)),
                    // operators
                    Token::Plus => operator_tokens.push(Operator::Add),
                    Token::Minus => operator_tokens.push(Operator::Sub),
                    Token::ArrayAccess => {
                        // accessing array from previous value token, coalesce
                        let array_value = value_tokens.pop().unwrap();
                        value_tokens.push(
                            Value::Expression {
                                values: vec![
                                    array_value,
                                    ASTGenerator::resolve_token_read_like_expression(vec![tokens[token_idx + 1].to_owned()])
                                ],
                                operators: vec![Operator::ArrayAccess],
                            }
                        );
                        token_idx += 1; // we consumed the next one too
                    },
                    _ => panic!("{:?} passed as value for variable read token!", this_token),
                }
            }

            token_idx += 1;
        }

        assert_eq!(value_tokens.len() - 1, operator_tokens.len());
        return Value::Expression {
            values: value_tokens,
            operators: operator_tokens
        };
    }
    fn resolve_variable_write_like_token(token: &Token) -> Value {
        match token {
            Token::Variable { name } => {
                Value::Variable(name.to_owned())
            }
            _ => {
                panic!("{:?} passed as value for variable write token!", token)
            }
        }
    }
    fn resolve_variable_name_like_token(token: &Token) -> Option<String> {
        match token {
            Token::Variable { name } => {
                Some(name.to_owned())
            }
            _ => {
                panic!("{:?} passed as value for variable write token!", token)
            }
        }
    }
    fn resolve_comparison_like_token(token: &Token) -> Operator {
        match token {
            Token::Equals => Operator::Equals,
            Token::NotEquals => Operator::NotEquals,
            Token::MoreThan => Operator::MoreThan,
            Token::LessThan => Operator::LessThan,
            Token::MoreThanOrEquals => Operator::MoreThanOrEquals,
            Token::LessThanOrEquals => Operator::LessThanOrEquals,
            _ => panic!("{:?} passed as value for comparison-like token!", token),
        }
    }
    fn resolve_any_value(&mut self) -> (Vec<Value>, Vec<Operator>) {
        let mut tokens: Vec<Token> = vec![];

        while
            !ASTGenerator::token_is_scope_like(self.peek_next_token().unwrap())
            && !ASTGenerator::token_is_line_end(self.peek_next_token().unwrap())
        {
            tokens.push(self.advance_and_get_token().to_owned());
        }

        if tokens.len() == 1 {
            // single literal
            return (vec![ASTGenerator::resolve_token_read_like_expression(tokens)], vec![])
        } else if tokens.len() > 1 {
            let mut array_scratch: Vec<Value> = vec![];
            let mut token_idx = 0;
            let mut value_tokens: Vec<Value> = vec![];
            let mut operator_tokens: Vec<Operator> = vec![];
    
            while token_idx < tokens.len() {
                if tokens[token_idx] == Token::ArrayOpen {
                    // handle array
                    token_idx += 1;
    
                    while tokens[token_idx] != Token::ArrayClose {
                        let this_token = tokens[token_idx].to_owned();
    
                        if ASTGenerator::token_is_assign_like(&this_token)
                        || ASTGenerator::token_is_assign_op_like(&this_token)
                        || ASTGenerator::token_is_comparison_like(&this_token)
                        || ASTGenerator::token_is_line_end(&this_token)
                        || ASTGenerator::token_is_scope_like(&this_token) {
                            panic!("Array incomplete!");
                        }
    
                        array_scratch.push(ASTGenerator::resolve_value_from_token(&this_token));
                        token_idx += 1;
                    }
    
                    value_tokens.push(Value::Array(array_scratch.to_owned()));
                    array_scratch.clear();
                } else if ASTGenerator::token_is_comparison_like(&tokens[token_idx]) {
                    // add new operator and move temp tokens to list of token lists
                    operator_tokens.push(ASTGenerator::resolve_comparison_like_token(&tokens[token_idx]));
                } else {
                    let this_token = tokens[token_idx].to_owned();
    
                    match this_token {
                        // values
                        Token::IntegerLiteral { value: _ } => value_tokens.push(ASTGenerator::resolve_value_from_token(&this_token)),
                        Token::StringLiteral { value: _ } => value_tokens.push(ASTGenerator::resolve_value_from_token(&this_token)),
                        Token::BoolTrue => value_tokens.push(ASTGenerator::resolve_value_from_token(&this_token)),
                        Token::BoolFalse => value_tokens.push(ASTGenerator::resolve_value_from_token(&this_token)),
                        Token::Variable { name: _ } => value_tokens.push(ASTGenerator::resolve_value_from_token(&this_token)),
                        // operators
                        Token::Plus => operator_tokens.push(Operator::Add),
                        Token::Minus => operator_tokens.push(Operator::Sub),
                        Token::ArrayAccess => {
                            // accessing array from previous value token, coalesce
                            let array_value = value_tokens.pop().unwrap();
                            let mut access_tokens: Vec<Token> = vec![];
                            // skip opening array access
                            token_idx += 1;

                            while tokens[token_idx] != Token::ArrayAccess {
                                access_tokens.push(tokens[token_idx].to_owned());
                                token_idx += 1;
                            }

                            // skip closing array access
                            token_idx += 1;
                            value_tokens.push(
                                Value::Expression {
                                    values: vec![
                                        array_value,
                                        ASTGenerator::resolve_token_read_like_expression(access_tokens)
                                    ],
                                    operators: vec![Operator::ArrayAccess],
                                }
                            );
                        },
                        _ => panic!("{:?} passed as value for variable read token!", this_token),
                    }
                }
    
                token_idx += 1;
            }

            return (value_tokens, operator_tokens);
        } else if false {
            let mut temp_tokens: Vec<Token> = vec![];
            let mut token_sublists: Vec<Vec<Token>> = vec![];
            let mut comparison_operators: Vec<Token> = vec![];

            for token in tokens {
                if ASTGenerator::token_is_comparison_like(&token) {
                    // add new operator and move temp tokens to list of token lists
                    comparison_operators.push(token.to_owned());
                    token_sublists.push(temp_tokens.to_owned());
                    temp_tokens.clear();
                } else {
                    temp_tokens.push(token.to_owned());
                }
            }

            if temp_tokens.len() > 0 {
                // get that last list
                token_sublists.push(temp_tokens.to_owned());
            }

            let mut final_values: Vec<Value> = vec![];
            let mut final_comparisons: Vec<Operator> = vec![];

            if comparison_operators.len() > 0 {
                for (index, token_sublist) in token_sublists.iter().enumerate() {
                    final_values.push(ASTGenerator::resolve_token_read_like_expression(
                        token_sublist.to_owned()
                    ));

                    if index > 0 {
                        final_comparisons.push(ASTGenerator::resolve_comparison_like_token(
                            comparison_operators.get(index - 1).unwrap()
                        ));
                    }
                }

                return (final_values, final_comparisons);
            } else {
                panic!("Invalid operand length!");
            }
        } else {
            panic!("Invalid operand length!");
        }
    }
    fn token_is_comparison_like(token: &Token) -> bool {
        match token {
            Token::Equals |
            Token::NotEquals |
            Token::MoreThan |
            Token::LessThan => true,
            _ => false,
        }
    }
    fn token_is_assign_op_like(token: &Token) -> bool {
        match token {
            Token::PlusEquals |
            Token::MinusEquals => true,
            _ => false,
        }
    }
    fn token_is_assign_like(token: &Token) -> bool {
        match token {
            Token::Assign => true,
            _ => false,
        }
    }
    fn token_is_scope_like(token: &Token) -> bool {
        match token {
            Token::ScopeOpen => true,
            _ => false,
        }
    }
    fn token_is_line_end(token: &Token) -> bool {
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
    fn insert_label(&mut self, new_label_name: String) {
        self.jump_table.insert(new_label_name.to_owned(), self.generated_ast.len() - 1);

        if !self.jump_table.contains_key(&new_label_name) {
            self.jump_table.insert(new_label_name.to_owned(), self.generated_ast.len() - 1);
        }

        let unorphaned_jumps = self.orphaned_jumps.get(&new_label_name);

        if unorphaned_jumps != None {
            for orphan_idx in unorphaned_jumps.unwrap() {
                self.generated_ast[*orphan_idx + 1] = ASTToken::of_type(
                    Statement::Jump(Some((self.generated_ast.len() - 1).to_owned()))
                )
            }

            self.orphaned_jumps.remove(&new_label_name);
        }
    }
    fn insert_jump_or_orphan(&mut self, jump_name: String) -> Option<usize> {
        // returns the index of the label to jump to, or None if the label is currently unknown
        let jump_index = self.jump_table.get(&jump_name);

        if jump_index == None {
            // orphan
            self.orphaned_jumps.entry(
                jump_name
            ).or_insert_with(
                Vec::new
            ).push(
                (self.generated_ast.len() - 1).to_owned()
            );

            None
        } else {
            Some(jump_index.unwrap().to_owned())
        }
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
                Token::Label => {
                    // create new label with name
                    let label_name = ASTGenerator::resolve_variable_name_like_token(
                        self.advance_and_get_token()
                    ).expect("Label name not passed to label!");

                    self.insert_ast_token_at_end(ASTToken::of_type(
                        Statement::Label(label_name.to_owned())
                    ));
                    self.insert_label(label_name);
                    // check for line end
                    assert_eq!(*self.peek_next_token().unwrap(), Token::LineEnd);
                }
                Token::Jump => {
                    // create new jump
                    let label_name = ASTGenerator::resolve_variable_name_like_token(
                        self.advance_and_get_token()
                    ).expect("Label name not passed to jump!");

                    let jump_idx = self.insert_jump_or_orphan(label_name);
                    self.insert_ast_token_at_end(ASTToken::of_type(
                        Statement::Jump(jump_idx)
                    ));
                    // check for line end
                    assert_eq!(*self.peek_next_token().unwrap(), Token::LineEnd);
                }
                Token::If => {
                    let (value_tokens, comparisons) = self.resolve_any_value();
                    let new_token: ASTToken;

                    if comparisons.len() == 0 {
                        // implicit bool
                        new_token = ASTToken::with_args_and_body(
                            Statement::If(Operator::Equals),
                            value_tokens[0].to_owned(),
                            Some(Value::BoolLiteral(true)),
                            self.generated_ast.len() + 1,
                            None,
                        );
                    } else {
                        new_token = ASTToken::with_args_and_body(
                            Statement::If(comparisons[0].to_owned()),
                            value_tokens[0].to_owned(),
                            Some(value_tokens[1].to_owned()),
                            self.generated_ast.len() + 1,
                            None,
                        );
                    }

                    // add new token to stack
                    self.insert_ast_token_at_end(new_token);
                    // check for block to execute after if statement
                    assert_eq!(*self.peek_next_token().unwrap(), Token::ScopeOpen);
                    self.insert_new_empty_ast_scope();
                    self.advance_token(); // skip scope open
                }
                Token::While => {
                    let (value_tokens, comparisons) = self.resolve_any_value();
                    let new_token: ASTToken;

                    if comparisons.len() == 0 {
                        // implicit bool
                        new_token = ASTToken::with_args_and_body(
                            Statement::While(Operator::Equals),
                            value_tokens[0].to_owned(),
                            Some(Value::BoolLiteral(true)),
                            self.generated_ast.len() + 1,
                            None,
                        );
                    } else {
                        new_token = ASTToken::with_args_and_body(
                            Statement::While(comparisons[0].to_owned()),
                            value_tokens[0].to_owned(),
                            Some(value_tokens[1].to_owned()),
                            self.generated_ast.len() + 1,
                            None,
                        );
                    }

                    // add new token to stack
                    self.insert_ast_token_at_end(new_token);
                    // check for block to execute after if statement
                    assert_eq!(*self.peek_next_token().unwrap(), Token::ScopeOpen);
                    self.insert_new_empty_ast_scope();
                    self.advance_token(); // skip scope open
                }
                Token::Alloc => {
                    // get the variable to assign to
                    let variable_expression: Value = ASTGenerator::resolve_variable_write_like_token(
                        self.advance_and_get_token()
                    );

                    // make sure the = is there
                    if !ASTGenerator::token_is_assign_like(self.advance_and_get_token()) {
                        panic!("{:?} passed as Assign to Alloc!", current_token)
                    }

                    let (value_tokens, comparisons) = self.resolve_any_value();
                    let new_token: ASTToken;

                    if comparisons.len() == 0 {
                        new_token = ASTToken::with_args_and_body(
                            Statement::Alloc,
                            variable_expression,
                            Some(value_tokens.get(0).unwrap().to_owned()),
                            self.generated_ast.len() + 1,
                            None,
                        );
                    } else {
                        new_token = ASTToken::with_args_and_body(
                            Statement::Alloc,
                            variable_expression,
                            Some(Value::Expression { values: value_tokens, operators: comparisons }),
                            self.generated_ast.len() + 1,
                            None,
                        );
                    }

                    self.insert_ast_token_at_end(new_token);
                    // check for line end, alloc takes a fixed amount of args
                    assert_eq!(*self.peek_next_token().unwrap(), Token::LineEnd);
                }
                Token::Set => {
                    // get the variable to assign to
                    let variable_expression: Value = ASTGenerator::resolve_variable_write_like_token(
                        self.advance_and_get_token()
                    );

                    // make sure the = is there
                    if !ASTGenerator::token_is_assign_like(self.advance_and_get_token()) {
                        panic!("{:?} passed as Assign to Set!", current_token)
                    }

                    let (value_tokens, comparisons) = self.resolve_any_value();
                    let new_token: ASTToken;

                    if comparisons.len() == 0 {
                        new_token = ASTToken::with_args_and_body(
                            Statement::Set,
                            variable_expression,
                            Some(value_tokens.get(0).unwrap().to_owned()),
                            self.generated_ast.len() + 1,
                            None,
                        );
                    } else {
                        new_token = ASTToken::with_args_and_body(
                            Statement::Set,
                            variable_expression,
                            Some(Value::Expression { values: value_tokens, operators: comparisons }),
                            self.generated_ast.len() + 1,
                            None,
                        );
                    }

                    self.insert_ast_token_at_end(new_token);
                    // check for line end, set takes a fixed amount of args
                    assert_eq!(*self.peek_next_token().unwrap(), Token::LineEnd);
                }
                Token::Variable { name } => {
                    let new_token: ASTToken;

                    if ASTGenerator::token_is_assign_op_like(self.peek_next_token().unwrap()) {
                        // plus and minus equals operators
                        let variable_expression: Value = ASTGenerator::resolve_variable_write_like_token(
                            &Token::Variable { name: name.to_owned() }
                        );
                        let assign_op: Token;
                        

                        match self.advance_and_get_token().to_owned() {
                            Token::PlusEquals => {
                                assign_op = Token::Plus;
                            }
                            Token::MinusEquals => {
                                assign_op = Token::Minus;
                            }
                            _ => {
                                unreachable!()
                            }
                        }

                        let mut new_value_tokens: Vec<Token> = vec![
                            Token::Variable { name: name.to_owned() },
                            assign_op
                        ];

                        while !ASTGenerator::token_is_line_end(self.peek_next_token().unwrap()) {
                            new_value_tokens.push(self.advance_and_get_token().to_owned());
                        }

                        let new_value_expression: Value = ASTGenerator::resolve_token_read_like_expression(
                            new_value_tokens
                        );
                        new_token = ASTToken::with_args(
                            Statement::Set,
                            variable_expression,
                            Some(new_value_expression),
                        );
                    } else {
                        panic!("Mysterious variable at start of statement with no assign operator or index access!")
                    }

                    self.insert_ast_token_at_end(new_token);
                    // check for line end, alloc takes a fixed amount of args
                    assert_eq!(*self.peek_next_token().unwrap(), Token::LineEnd);
                }
                Token::Print => {
                    // debug printing, takes 1 variable-like argument
                    let (value_tokens, comparisons) = self.resolve_any_value();
                    let new_token: ASTToken;

                    if comparisons.len() == 0 {
                        new_token = ASTToken::with_args(
                            Statement::DebugPrintCall,
                            value_tokens.get(0).unwrap().to_owned(),
                            None,
                        );
                    } else {
                        new_token = ASTToken::with_args(
                            Statement::DebugPrintCall,
                            Value::Expression { values: value_tokens, operators: comparisons },
                            None,
                        );
                    }

                    self.insert_ast_token_at_end(new_token);
                    assert_eq!(*self.peek_next_token().unwrap(), Token::LineEnd);
                }
                Token::ReadLine => {
                    // read line of input from terminal, takes 1 variable argument
                    let variable_expression: Value = ASTGenerator::resolve_variable_write_like_token(
                        self.advance_and_get_token()
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
