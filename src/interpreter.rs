use std::collections::HashMap;

use crate::astgen::{ASTToken, Operator, Statement, Value};

#[derive(Debug, Clone, PartialEq)]
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
            self.create_new_variable(name, value);
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
    fn resolve_argument_value(&self, argument: Value) -> Type {
        if let Value::Variable(name) = argument {
            self.resolve_variable_by_name(name)
        } else {
            match argument {
                Value::IntegerLiteral(value) => Type::Integer(value),
                Value::StringLiteral(value) => Type::String(value),
                Value::BoolLiteral(value) => Type::Bool(value),
                Value::Variable(name) => self.resolve_variable_by_name(name),
                Value::Expression { values, operators } => {
                    // oh boy
                    let mut accumulator: Type = self.resolve_argument_value(
                        values.first().unwrap().clone()
                    );
                    let mut index = 0;

                    for operator in operators {
                        accumulator = Interpreter::operate_on_types(
                            accumulator.clone(),
                            self.resolve_argument_value(values.get(index + 1).unwrap().clone()),
                            operator
                        );
                        index += 1;
                    }

                    accumulator
                }
                _ => panic!("Malformed argument expression!"),
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
    fn operate_on_types(first: Type, second: Type, operator: Operator) -> Type {
        match &first {
            Type::Bool(first_val) => {
                match &second {
                    Type::Bool(second_val) => {
                        match operator {
                            // logical
                            Operator::Equals => { return Type::Bool(first_val == second_val); }
                            Operator::NotEquals => { return Type::Bool(first_val != second_val); }
                            Operator::MoreThan => { return Type::Bool(first_val > second_val); }
                            Operator::LessThan => { return Type::Bool(first_val < second_val); }
                            Operator::MoreThanOrEquals => { return Type::Bool(first_val >= second_val); }
                            Operator::LessThanOrEquals => { return Type::Bool(first_val <= second_val); }
                            _ => panic!("Invalid operator for comparison statement: {:?}", operator)
                        }
                    }
                    _ => {
                        panic!("Invalid args for comparison statement: {:?} | {:?}", first, second);
                    }
                }
            }
            Type::Integer(first_val) => {
                match &second {
                    Type::Integer(second_val) => {
                        match operator {
                            // math
                            Operator::Add => { return Type::Integer(first_val + second_val); }
                            Operator::Sub => { return Type::Integer(first_val - second_val); }
                            // logical
                            Operator::Equals => { return Type::Bool(first_val == second_val); }
                            Operator::NotEquals => { return Type::Bool(first_val != second_val); }
                            Operator::MoreThan => { return Type::Bool(first_val > second_val); }
                            Operator::LessThan => { return Type::Bool(first_val < second_val); }
                            Operator::MoreThanOrEquals => { return Type::Bool(first_val >= second_val); }
                            Operator::LessThanOrEquals => { return Type::Bool(first_val <= second_val); }
                            _ => panic!("Invalid operator for comparison statement: {:?}", operator)
                        }
                    }
                    Type::Bool(second_val) => {
                        match operator {
                            // logical
                            Operator::Equals => { return Type::Bool((*first_val == 0) != *second_val); }
                            Operator::NotEquals => { return Type::Bool((*first_val != 0) != *second_val); }
                            _ => panic!("Invalid operator for comparison statement: {:?}", operator)
                        }
                    }
                    _ => {
                        panic!("Invalid args for comparison statement: {:?} | {:?}", first, second);
                    }
                }
            }
            Type::String(first_val) => {
                match &second {
                    Type::Integer(second_val) => {
                        match operator {
                            // math
                            Operator::Add => { return Type::String(first_val.to_string() + &second_val.to_string()); }
                            _ => panic!("Invalid operator for comparison statement: {:?}", operator)
                        }
                    }
                    Type::Bool(second_val) => {
                        match operator {
                            // math
                            Operator::Add => { return Type::String(first_val.to_string() + &second_val.to_string()); }
                            // logical
                            Operator::Equals => { return Type::Bool((first_val.len() == 0) != *second_val); }
                            Operator::NotEquals => { return Type::Bool((first_val.len() != 0) != *second_val); }
                            _ => panic!("Invalid operator for comparison statement: {:?}", operator)
                        }
                    }
                    Type::String(second_val) => {
                        match operator {
                            // math
                            Operator::Add => { return Type::String(first_val.to_string() + second_val); }
                            // logical
                            Operator::Equals => { return Type::Bool(first_val == second_val); }
                            Operator::NotEquals => { return Type::Bool(first_val != second_val); }
                            Operator::MoreThan => { return Type::Bool(first_val.len() > second_val.len()); }
                            Operator::LessThan => { return Type::Bool(first_val.len() < second_val.len()); }
                            Operator::MoreThanOrEquals => { return Type::Bool(first_val.len() >= second_val.len()); }
                            Operator::LessThanOrEquals => { return Type::Bool(first_val.len() <= second_val.len()); }
                            _ => panic!("Invalid operator for comparison statement: {:?}", operator)
                        }
                    }
                }
            }
            _ => {
                panic!("Operating on a type that can't be operated on!");
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
                t_type: Statement::Block,
                arg1: _,
                arg2: _,
                body_idx: _,
                body_extent: _,
                else_body_idx: _,
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
            } => {
                let loop_idx = self.loop_stack.pop().unwrap() - 1;
                self.invalidate_current_scope();
                let previous_token = self.get_inst(loop_idx);

                if loop_idx > 0 {
                    if let ASTToken {
                        t_type: Statement::While(_),
                        arg1: _,
                        arg2: _,
                        body_idx: _,
                        body_extent: _,
                        else_body_idx: _,
                    } = previous_token {
                        self.inst_ptr = loop_idx;
                    } else {
                        self.inst_ptr += 1;
                    }
                }
            }
            ASTToken {
                t_type: Statement::Alloc,
                arg1,
                arg2,
                body_idx: _,
                body_extent: _,
                else_body_idx: _,
            } => {
                if let Some(Value::Variable(name)) = arg1 {
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
                if let Some(Value::Variable(name)) = arg1 {
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
                    Type::Integer(value) => print!("{}", value),
                    Type::String(value) => print!("{}", value.replace("\\n", "\n")), // jank shit
                    Type::Bool(value) => print!("{}", value),
                }

                self.inst_ptr += 1;
            }
            ASTToken {
                t_type: Statement::Jump(jump_idx),
                arg1: _,
                arg2: _,
                body_idx: _,
                body_extent: _,
                else_body_idx: _,
            } => {
                self.inst_ptr = jump_idx.unwrap();
            }
            ASTToken {
                t_type: Statement::If(comparison_operator),
                arg1,
                arg2,
                body_idx: _,
                body_extent: _,
                else_body_idx: _,
            } => {
                let first_arg: Type = self.resolve_argument_value(arg1.unwrap());
                let second_arg: Type = self.resolve_argument_value(arg2.unwrap());

                if Interpreter::operate_on_types(first_arg, second_arg, comparison_operator) == Type::Bool(true) {
                    self.inst_ptr += 1;
                } else {
                    // skip scope open and close at least
                    self.inst_ptr += self.peek_next_inst().body_extent.unwrap() + 2;
                }
            }
            ASTToken {
                t_type: Statement::While(comparison_operator),
                arg1,
                arg2,
                body_idx: _,
                body_extent: _,
                else_body_idx: _,
            } => {
                let first_arg: Type = self.resolve_argument_value(arg1.unwrap());
                let second_arg: Type = self.resolve_argument_value(arg2.unwrap());

                if Interpreter::operate_on_types(first_arg, second_arg, comparison_operator) == Type::Bool(true) {
                    self.inst_ptr += 1;
                } else {
                    // skip scope open and close at least
                    self.inst_ptr += self.peek_next_inst().body_extent.unwrap() + 2;
                }
            }
            _ => {
                self.inst_ptr += 1;
            }
        }
    }
}
