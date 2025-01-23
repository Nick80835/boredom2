use std::collections::HashMap;

use crate::astgen::{ASTToken, Expression, Statement};

#[derive(Debug, Clone)]
enum Type {
    Integer(u32),
    String(String),
}

pub struct Interpreter {
    pub ast_tokens: Vec<ASTToken>,
    pub halted: bool,
    inst_ptr: usize,
    memory_cells: Vec<Type>,
    alloc_ptr: usize,
    variable_map: HashMap<String, usize>,
    scope_start_stack: Vec<usize>,
}

impl Interpreter {
    pub fn init(ast_tokens: Vec<ASTToken>) -> Self {
        Self {
            ast_tokens,
            halted: false,
            inst_ptr: 0,
            memory_cells: vec![],
            alloc_ptr: 0,
            variable_map: HashMap::new(),
            scope_start_stack: vec![0],
        }
    }
    fn current_inst(&self) -> &ASTToken {
        &self.ast_tokens[self.inst_ptr]
    }
    fn peek_next_inst(&self) -> &ASTToken {
        &self.ast_tokens[self.inst_ptr + 1]
    }
    pub fn print_state(&self) {
        println!("STATE\n{} | {:?}\n{:?}", self.inst_ptr, self.current_inst(), self.memory_cells);
    }
    fn create_new_variable(&mut self, name: String, value: Type) {
        if self.variable_map.get(&name) != None {
            panic!("Trying to allocate a variable '{}' that already exists!", name);
        }
        self.variable_map.insert(name, self.memory_cells.len());
        self.memory_cells.push(value);
    }
    fn set_or_create_new_variable(&mut self, name: String, value: Type) {
        let existing_idx = self.variable_map.get(&name);

        if existing_idx != None {
            self.memory_cells[*existing_idx.unwrap()] = value;
        } else {
            self.variable_map.insert(name, self.memory_cells.len());
            self.memory_cells.push(value);
        }
    }
    fn resolve_variable_by_name(&self, name: String) -> Type {
        let addr = self.variable_map.get(&name);

        if addr == None {
            panic!("Unknown variable name!");
        }

        let var = &self.memory_cells[*addr.unwrap()];
        return var.to_owned();
    }
    fn resolve_argument_value(&self, argument: Expression) -> Type {
        if let Expression::Variable(name) = argument {
            self.resolve_variable_by_name(name)
        } else {
            match argument {
                Expression::IntegerLiteral(value) => {
                    Type::Integer(value)
                }
                Expression::StringLiteral(value) => {
                    Type::String(value)
                }
                _ => {
                    panic!("Malformed argument expression!")
                }
            }
        }
    }

    pub fn execute_one(&mut self) {
        let current_instruction = self.current_inst().to_owned();

        match current_instruction {
            ASTToken {
                t_type: Statement::EOF,
                arg1: _,
                arg2: _,
                body_idx: _,
                body_extent: _,
                else_body_idx: _,
            } => {
                self.halted = true;
            }
            ASTToken {
                t_type: Statement::Allocate,
                arg1,
                arg2,
                body_idx: _,
                body_extent: _,
                else_body_idx: _,
            } => {
                if let Some(Expression::Variable(name)) = arg1 {
                    self.create_new_variable(
                        name.to_owned(),
                        self.resolve_argument_value(arg2.unwrap())
                    );
                } else {
                    panic!("Malformed allocate!")
                }

                self.inst_ptr += 1;
            }
            ASTToken {
                t_type: Statement::Set,
                arg1,
                arg2,
                body_idx: _,
                body_extent: _,
                else_body_idx: _,
            } => {
                if let Some(Expression::Variable(name)) = arg1 {
                    self.set_or_create_new_variable(
                        name.to_owned(),
                        self.resolve_argument_value(arg2.unwrap())
                    );
                } else {
                    panic!("Malformed set!")
                }

                self.inst_ptr += 1;
            }
            ASTToken {
                t_type: Statement::DebugPrintCall,
                arg1,
                arg2: _,
                body_idx: _,
                body_extent: _,
                else_body_idx: _,
            } => {
                match self.resolve_argument_value(arg1.unwrap()) {
                    Type::Integer(value) => println!("{}", value),
                    Type::String(value) => println!("{}", value),
                }

                self.inst_ptr += 1;
            }
            ASTToken {
                t_type: Statement::Equals,
                arg1,
                arg2,
                body_idx: _,
                body_extent: _,
                else_body_idx: _,
            } => {
                match self.resolve_argument_value(arg1.unwrap()) {
                    Type::Integer(value) => {
                        match self.resolve_argument_value(arg2.unwrap()) {
                            Type::Integer(value2) => {
                                if value == value2 {
                                    self.inst_ptr += 1;
                                } else {
                                     // skip scope open and close at least
                                    self.inst_ptr += self.peek_next_inst().body_extent.unwrap() + 2;
                                }
                            },
                            arg => panic!("Invalid arg for == statement: {:?} == {:?}", value, arg),
                        }
                    },
                    arg => panic!("Invalid arg for == statement: {:?}", arg),
                }
            }
            _ => {
                self.inst_ptr += 1;
            }
        }
    }
}
