pub struct Interpreter {
    tokens: Vec<Token>,
    inst_ptr: usize,
    memory_cells: [u32; 8],
}

impl Interpreter {
    pub fn init(tokens: Vec<Token>) -> Self { Self { tokens, inst_ptr: 0, memory_cells: [0; 8] } }
    fn current_inst(&self) -> &Token { &self.tokens[self.inst_ptr] }

    pub fn print_state(&self) {
        println!("STATE\n{}\n{:?}", self.inst_ptr, self.current_inst());
    }

    pub fn execute_one(&mut self) {

    }
}
