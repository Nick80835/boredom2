use std::collections::HashMap;

use crate::astgen::{ASTToken, Expression, Statement};

#[derive(Debug, Clone)]
enum Type {
    Integer(u32),
    String(String),
    Bool(bool),
}

pub struct Interpreter {
    pub ast_tokens: Vec<ASTToken>,
    pub halted: bool,
    inst_ptr: usize,
    memory_cells: Vec<Type>,
    variable_map: HashMap<String, usize>,
    mem_scope_start_stack: Vec<usize>,
    loop_stack: Vec<usize>,
    return_stack: Vec<usize>,
}

impl Interpreter {
    pub fn init(ast_tokens: Vec<ASTToken>) -> Self {
        Self {
            ast_tokens,
            halted: false,
            inst_ptr: 0,
            memory_cells: vec![],
            variable_map: HashMap::new(),
            mem_scope_start_stack: vec![0],
            loop_stack: vec![],
            return_stack: vec![],
        }
    }
    fn current_inst(&self) -> &ASTToken {
        &self.ast_tokens[self.inst_ptr]
    }
    fn peek_next_inst(&self) -> &ASTToken {
        &self.ast_tokens[self.inst_ptr + 1]
    }
    fn get_inst(&self, idx: usize) -> &ASTToken {
        self.ast_tokens.get(idx).unwrap()
    }
    pub fn print_state(&self) {
        println!("STATE\n{} | {:?}\n{:?}\n{:?}", self.inst_ptr, self.current_inst(), self.memory_cells, self.variable_map);
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
                Expression::Bool(value) => {
                    Type::Bool(value)
                }
                _ => {
                    panic!("Malformed argument expression!")
                }
            }
        }
    }
    fn invalidate_current_scope(&mut self) {
        if self.memory_cells.len() > 0 {
            let invalid_scope_start: usize = self.mem_scope_start_stack.pop().unwrap() - 1;
            self.variable_map.retain(
                |_, v| *v <= invalid_scope_start
            );
            self.memory_cells.truncate(invalid_scope_start + 1);
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
                recurring: _,
            } => {
                self.halted = true;
            }
            ASTToken {
                t_type: Statement::Block,
                arg1: _,
                arg2: _,
                body_idx: _,
                body_extent: _,
                else_body_idx: _,
                recurring: _,
            } => {
                self.mem_scope_start_stack.push(self.memory_cells.len());
                self.loop_stack.push(self.inst_ptr);
                self.inst_ptr += 1;
            }
            ASTToken {
                t_type: Statement::BlockEnd,
                arg1: _,
                arg2: _,
                body_idx: _,
                body_extent: _,
                else_body_idx: _,
                recurring: _,
            } => {
                let loop_idx = self.loop_stack.pop().unwrap() - 1;
                self.invalidate_current_scope();
                let previous_token = self.get_inst(loop_idx);

                if loop_idx > 0 {
                    if let ASTToken {
                        t_type: _,
                        arg1: _,
                        arg2: _,
                        body_idx: _,
                        body_extent: _,
                        else_body_idx: _,
                        recurring: true,
                    } = previous_token {
                        self.inst_ptr = loop_idx;
                    } else {
                        self.inst_ptr += 1;
                    }
                }
            }
            ASTToken {
                t_type: Statement::Allocate,
                arg1,
                arg2,
                body_idx: _,
                body_extent: _,
                else_body_idx: _,
                recurring: _,
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
                recurring: _,
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
                recurring: _,
            } => {
                match self.resolve_argument_value(arg1.unwrap()) {
                    Type::Integer(value) => println!("{}", value),
                    Type::String(value) => println!("{}", value),
                    Type::Bool(value) => println!("{}", value),
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
                recurring: _,
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
                    Type::Bool(value) => {
                        match self.resolve_argument_value(arg2.unwrap()) {
                            Type::Bool(value2) => {
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
