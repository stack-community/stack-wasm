use wasm_bindgen::prelude::*;
use web_sys::js_sys::Function;
use web_sys::{Document, Element};

#[wasm_bindgen]
pub fn run_stack(src: &str) -> Result {
    let mut executor = Executor::new();
    executor.evaluate_program(src.to_string());
    Result::new(executor.output, executor.log)
}

fn get_element_by_id(element_id: String) -> Type {
    let window: web_sys::Window = web_sys::window().expect("no global `window` exists");
    let document: Document = window.document().expect("should have a document on window");

    if let Some(element) = document.get_element_by_id(&element_id) {
        Type::Element(element)
    } else {
        Type::Error("element-not-found".to_string())
    }
}

#[wasm_bindgen]
extern "C" {
    pub fn prompt(s: &str) -> String;
}

#[wasm_bindgen]
pub struct Result {
    output: String,
    log: String,
}

#[wasm_bindgen]
impl Result {
    pub fn new(output: String, log: String) -> Self {
        Result { output, log }
    }

    pub fn output(&self) -> String {
        self.output.clone()
    }

    pub fn log(&self) -> String {
        self.log.clone()
    }
}

use std::collections::HashMap;
use std::env;

/// Data type
#[derive(Clone, Debug)]
enum Type {
    Number(f64),
    String(String),
    Bool(bool),
    List(Vec<Type>),
    Object(String, HashMap<String, Type>),
    Error(String),
    Element(Element),
}

/// Implement methods
impl Type {
    /// Show data to display
    fn display(&self) -> String {
        match self {
            Type::Number(num) => num.to_string(),
            Type::String(s) => format!("({})", s),
            Type::Bool(b) => b.to_string(),
            Type::List(list) => {
                let result: Vec<String> = list.iter().map(|token| token.display()).collect();
                format!("[{}]", result.join(" "))
            }
            Type::Error(err) => format!("error:{err}"),
            Type::Object(name, _) => format!("Object<{name}>"),
            Type::Element(node) => node.to_string().into(),
        }
    }

    /// Get string form data
    fn get_string(&mut self) -> String {
        match self {
            Type::String(s) => s.to_string(),
            Type::Number(i) => i.to_string(),
            Type::Bool(b) => b.to_string(),
            Type::List(l) => Type::List(l.to_owned()).display(),
            Type::Error(err) => format!("error:{err}"),
            Type::Object(name, _) => format!("Object<{name}>"),
            Type::Element(node) => node.to_string().into(),
        }
    }

    /// Get number from data
    fn get_number(&mut self) -> f64 {
        match self {
            Type::String(s) => s.parse().unwrap_or(0.0),
            Type::Number(i) => *i,
            Type::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            Type::List(l) => l.len() as f64,
            Type::Error(e) => e.parse().unwrap_or(0f64),
            Type::Object(_, object) => object.len() as f64,
            Type::Element(_) => 0f64,
        }
    }

    /// Get bool from data
    fn get_bool(&mut self) -> bool {
        match self {
            Type::String(s) => !s.is_empty(),
            Type::Number(i) => *i != 0.0,
            Type::Bool(b) => *b,
            Type::List(l) => !l.is_empty(),
            Type::Error(e) => e.parse().unwrap_or(false),
            Type::Object(_, object) => object.is_empty(),
            Type::Element(_) => false,
        }
    }

    /// Get list form data
    fn get_list(&mut self) -> Vec<Type> {
        match self {
            Type::String(s) => s
                .to_string()
                .chars()
                .map(|x| Type::String(x.to_string()))
                .collect::<Vec<Type>>(),
            Type::Number(i) => vec![Type::Number(*i)],
            Type::Bool(b) => vec![Type::Bool(*b)],
            Type::List(l) => l.to_vec(),
            Type::Error(e) => vec![Type::Error(e.to_string())],
            Type::Object(_, object) => object.values().map(|x| x.to_owned()).collect::<Vec<Type>>(),
            Type::Element(_) => vec![],
        }
    }

    fn get_object(&self) -> (String, HashMap<String, Type>) {
        match self {
            Type::Object(name, value) => (name.to_owned(), value.to_owned()),
            _ => ("".to_string(), HashMap::new()),
        }
    }

    fn get_element(&self) -> Element {
        match self {
            Type::Element(i) => i.clone(),
            _ => panic!("It's not element type"),
        }
    }
}

/// Manage program execution
#[derive(Clone, Debug)]
struct Executor {
    stack: Vec<Type>,              // Data stack
    memory: HashMap<String, Type>, // Variable's memory
    output: String,
    log: String,
}

impl Executor {
    /// Constructor
    fn new() -> Executor {
        Executor {
            stack: Vec::new(),
            memory: HashMap::new(),
            output: String::new(),
            log: String::new(),
        }
    }

    // Log
    fn log(&mut self, msg: String) {
        self.log += &format!("{msg}")
    }

    // Print to standard output
    fn print(&mut self, msg: String) {
        self.output += &format!("{msg}\n")
    }

    /// Show variable inside memory
    fn show_variables(&mut self) {
        self.log("Variables {\n".to_string());
        let max = self.memory.keys().map(|s| s.len()).max().unwrap_or(0);
        for (name, value) in self.memory.clone() {
            self.log(format!(
                " {:>width$}: {}\n",
                name,
                value.display(),
                width = max
            ))
        }
        self.log("}\n".to_string())
    }

    /// Show inside the stack
    fn show_stack(&mut self) -> String {
        format!(
            "Stack〔 {} 〕",
            self.stack
                .iter()
                .map(|x| x.display())
                .collect::<Vec<_>>()
                .join(" | ")
        )
    }

    /// Parse token by analyzing syntax
    fn analyze_syntax(&mut self, code: String) -> Vec<String> {
        // Convert tabs, line breaks, and full-width spaces to half-width spaces
        let code = code.replace(['\n', '\t', '\r', '　'], " ");

        let mut syntax = Vec::new(); // Token string
        let mut buffer = String::new(); // Temporary storage
        let mut in_brackets = 0; // String's nest structure
        let mut in_parentheses = 0; // List's nest structure
        let mut in_hash = false; // Is it Comment

        for c in code.chars() {
            match c {
                '(' => {
                    in_brackets += 1;
                    buffer.push('(');
                }
                ')' => {
                    in_brackets -= 1;
                    buffer.push(')');
                }
                '#' if !in_hash => {
                    in_hash = true;
                    buffer.push('#');
                }
                '#' if in_hash => {
                    in_hash = false;
                    buffer.push('#');
                }
                '[' if in_brackets == 0 => {
                    in_parentheses += 1;
                    buffer.push('[');
                }
                ']' if in_brackets == 0 => {
                    in_parentheses -= 1;
                    buffer.push(']');
                }
                ' ' if !in_hash && in_parentheses == 0 && in_brackets == 0 => {
                    if !buffer.is_empty() {
                        syntax.push(buffer.clone());
                        buffer.clear();
                    }
                }
                _ => {
                    buffer.push(c);
                }
            }
        }

        if !buffer.is_empty() {
            syntax.push(buffer);
        }
        syntax
    }

    /// evaluate string as program
    fn evaluate_program(&mut self, code: String) {
        // Parse into token string
        let syntax: Vec<String> = self.analyze_syntax(code);

        for token in syntax {
            // Show inside stack to debug
            let stack = self.show_stack();
            self.log(format!("{} ←  {}\n", stack, token));

            // Character vector for token processing
            let chars: Vec<char> = token.chars().collect();

            // Judge what the token is
            if let Ok(i) = token.parse::<f64>() {
                // Push number value on the stack
                self.stack.push(Type::Number(i));
            } else if token == "true" || token == "false" {
                // Push bool value on the stack
                self.stack.push(Type::Bool(token.parse().unwrap_or(true)));
            } else if chars[0] == '(' && chars[chars.len() - 1] == ')' {
                // Push string value on the stack
                self.stack
                    .push(Type::String(token[1..token.len() - 1].to_string()));
            } else if chars[0] == '[' && chars[chars.len() - 1] == ']' {
                // Push list value on the stack
                let old_len = self.stack.len(); // length of old stack
                let slice = &token[1..token.len() - 1];
                self.evaluate_program(slice.to_string());
                // Make increment of stack an element of list
                let mut list = Vec::new();
                for _ in old_len..self.stack.len() {
                    list.push(self.pop_stack());
                }
                list.reverse(); // reverse list
                self.stack.push(Type::List(list));
            } else if token.starts_with("error:") {
                // Push error value on the stack
                self.stack.push(Type::Error(token.replace("error:", "")))
            } else if let Some(i) = self.memory.get(&token) {
                // Push variable's data on stack
                self.stack.push(i.clone());
            } else if chars[0] == '#' && chars[chars.len() - 1] == '#' {
                // Processing comments
                self.log(format!("* Comment \"{}\"\n", token.replace('#', "")));
            } else {
                // Else, execute as command
                self.execute_command(token);
            }
        }

        // Show inside stack, after execution
        let stack = self.show_stack();
        self.log(format!("{}\n", stack));
    }

    /// execute string as commands
    fn execute_command(&mut self, command: String) {
        match command.as_str() {
            // Commands of calculation

            // addition
            "add" => {
                let b = self.pop_stack().get_number();
                let a = self.pop_stack().get_number();
                self.stack.push(Type::Number(a + b));
            }

            // Subtraction
            "sub" => {
                let b = self.pop_stack().get_number();
                let a = self.pop_stack().get_number();
                self.stack.push(Type::Number(a - b));
            }

            // Multiplication
            "mul" => {
                let b = self.pop_stack().get_number();
                let a = self.pop_stack().get_number();
                self.stack.push(Type::Number(a * b));
            }

            // Division
            "div" => {
                let b = self.pop_stack().get_number();
                let a = self.pop_stack().get_number();
                self.stack.push(Type::Number(a / b));
            }

            // Remainder of division
            "mod" => {
                let b = self.pop_stack().get_number();
                let a = self.pop_stack().get_number();
                self.stack.push(Type::Number(a % b));
            }

            // Exponentiation
            "pow" => {
                let b = self.pop_stack().get_number();
                let a = self.pop_stack().get_number();
                self.stack.push(Type::Number(a.powf(b)));
            }

            // Rounding off
            "round" => {
                let a = self.pop_stack().get_number();
                self.stack.push(Type::Number(a.round()));
            }

            // Trigonometric sine
            "sin" => {
                let number = self.pop_stack().get_number();
                self.stack.push(Type::Number(number.sin()))
            }

            // Trigonometric cosine
            "cos" => {
                let number = self.pop_stack().get_number();
                self.stack.push(Type::Number(number.cos()))
            }

            // Trigonometric tangent
            "tan" => {
                let number = self.pop_stack().get_number();
                self.stack.push(Type::Number(number.tan()))
            }

            // Logical operations of AND
            "and" => {
                let b = self.pop_stack().get_bool();
                let a = self.pop_stack().get_bool();
                self.stack.push(Type::Bool(a && b));
            }

            // Logical operations of OR
            "or" => {
                let b = self.pop_stack().get_bool();
                let a = self.pop_stack().get_bool();
                self.stack.push(Type::Bool(a || b));
            }

            // Logical operations of NOT
            "not" => {
                let b = self.pop_stack().get_bool();
                self.stack.push(Type::Bool(!b));
            }

            // Is it equal
            "equal" => {
                let b = self.pop_stack().get_string();
                let a = self.pop_stack().get_string();
                self.stack.push(Type::Bool(a == b));
            }

            // Is it less
            "less" => {
                let b = self.pop_stack().get_number();
                let a = self.pop_stack().get_number();
                self.stack.push(Type::Bool(a < b));
            }

            // Commands of string processing

            // Repeat string a number of times
            "repeat" => {
                let count = self.pop_stack().get_number(); // 回数
                let text = self.pop_stack().get_string(); // 文字列
                self.stack.push(Type::String(text.repeat(count as usize)));
            }

            // Get unicode character form number
            "decode" => {
                let code = self.pop_stack().get_number();
                let result = char::from_u32(code as u32);
                match result {
                    Some(c) => self.stack.push(Type::String(c.to_string())),
                    None => {
                        self.log("Error! failed of number decoding\n".to_string());
                        self.stack.push(Type::Error("number-decoding".to_string()));
                    }
                }
            }

            // Encode string by UTF-8
            "encode" => {
                let string = self.pop_stack().get_string();
                if let Some(first_char) = string.chars().next() {
                    self.stack.push(Type::Number((first_char as u32) as f64));
                } else {
                    self.log("Error! failed of string encoding\n".to_string());
                    self.stack.push(Type::Error("string-encoding".to_string()));
                }
            }

            // Concatenate the string
            "concat" => {
                let b = self.pop_stack().get_string();
                let a = self.pop_stack().get_string();
                self.stack.push(Type::String(a + &b));
            }

            // Replacing string
            "replace" => {
                let after = self.pop_stack().get_string();
                let before = self.pop_stack().get_string();
                let text = self.pop_stack().get_string();
                self.stack.push(Type::String(text.replace(&before, &after)))
            }

            // split string by key
            "split" => {
                let key = self.pop_stack().get_string();
                let text = self.pop_stack().get_string();
                self.stack.push(Type::List(
                    text.split(&key)
                        .map(|x| Type::String(x.to_string()))
                        .collect::<Vec<Type>>(),
                ));
            }

            // Generate a string by concat list
            "join" => {
                let key = self.pop_stack().get_string();
                let mut list = self.pop_stack().get_list();
                self.stack.push(Type::String(
                    list.iter_mut()
                        .map(|x| x.get_string())
                        .collect::<Vec<String>>()
                        .join(&key),
                ))
            }

            // Is it finding in string
            "find" => {
                let word = self.pop_stack().get_string();
                let text = self.pop_stack().get_string();
                self.stack.push(Type::Bool(text.contains(&word)))
            }

            // Commands of I/O

            // Standard input
            "input" => {
                let promp = self.pop_stack().get_string();
                self.stack.push(Type::String(prompt(promp.as_str())));
            }

            // Standard output
            "print" => {
                let a = self.pop_stack().get_string();
                self.print(format!("{a}"));
            }

            // Get command-line arguments
            "args-cmd" => self.stack.push(Type::List(
                env::args()
                    .collect::<Vec<_>>()
                    .iter()
                    .map(|x| Type::String(x.to_string()))
                    .collect::<Vec<Type>>(),
            )),

            // Commands of control

            // evaluate string as program
            "eval" => {
                let code = self.pop_stack().get_string();
                self.evaluate_program(code)
            }

            // Conditional branch
            "if" => {
                let condition = self.pop_stack().get_bool(); // condition
                let code_else = self.pop_stack().get_string(); // else code
                let code_if = self.pop_stack().get_string(); // if code
                if condition {
                    self.evaluate_program(code_if)
                } else {
                    self.evaluate_program(code_else)
                };
            }

            // Loop while condition is true
            "while" => {
                let cond = self.pop_stack().get_string();
                let code = self.pop_stack().get_string();
                while {
                    self.evaluate_program(cond.clone());
                    self.pop_stack().get_bool()
                } {
                    self.evaluate_program(code.clone());
                }
            }

            // Commands of list processing

            // Get list value by index
            "get" => {
                let index = self.pop_stack().get_number() as usize;
                let list: Vec<Type> = self.pop_stack().get_list();
                if list.len() > index {
                    self.stack.push(list[index].clone());
                } else {
                    self.log("Error! Index specification is out of range\n".to_string());
                    self.stack.push(Type::Error("index-out-range".to_string()));
                }
            }

            // Set list value by index
            "set" => {
                let value = self.pop_stack();
                let index = self.pop_stack().get_number() as usize;
                let mut list: Vec<Type> = self.pop_stack().get_list();
                if list.len() > index {
                    list[index] = value;
                    self.stack.push(Type::List(list));
                } else {
                    self.log("Error! Index specification is out of range\n".to_string());
                    self.stack.push(Type::Error("index-out-range".to_string()));
                }
            }

            // Delete list value by index
            "del" => {
                let index = self.pop_stack().get_number() as usize;
                let mut list = self.pop_stack().get_list();
                if list.len() > index {
                    list.remove(index);
                    self.stack.push(Type::List(list));
                } else {
                    self.log("Error! Index specification is out of range\n".to_string());
                    self.stack.push(Type::Error("index-out-range".to_string()));
                }
            }

            // Append value in the list
            "append" => {
                let data = self.pop_stack();
                let mut list = self.pop_stack().get_list();
                list.push(data);
                self.stack.push(Type::List(list));
            }

            // Insert value in the list
            "insert" => {
                let data = self.pop_stack();
                let index = self.pop_stack().get_number();
                let mut list = self.pop_stack().get_list();
                list.insert(index as usize, data);
                self.stack.push(Type::List(list));
            }

            // Sorting in the list
            "sort" => {
                let mut list: Vec<String> = self
                    .pop_stack()
                    .get_list()
                    .iter()
                    .map(|x| x.to_owned().get_string())
                    .collect();
                list.sort();
                self.stack.push(Type::List(
                    list.iter()
                        .map(|x| Type::String(x.to_string()))
                        .collect::<Vec<_>>(),
                ));
            }

            // reverse in the list
            "reverse" => {
                let mut list = self.pop_stack().get_list();
                list.reverse();
                self.stack.push(Type::List(list));
            }

            // Iteration
            "for" => {
                let code = self.pop_stack().get_string();
                let vars = self.pop_stack().get_string();
                let list = self.pop_stack().get_list();

                list.iter().for_each(|x| {
                    self.memory
                        .entry(vars.clone())
                        .and_modify(|value| *value = x.clone())
                        .or_insert(x.clone());
                    self.evaluate_program(code.clone());
                });
            }

            // Mapping a list
            "map" => {
                let code = self.pop_stack().get_string();
                let vars = self.pop_stack().get_string();
                let list = self.pop_stack().get_list();

                let mut result_list = Vec::new();
                for x in list.iter() {
                    self.memory
                        .entry(vars.clone())
                        .and_modify(|value| *value = x.clone())
                        .or_insert(x.clone());

                    self.evaluate_program(code.clone());
                    result_list.push(self.pop_stack());
                }

                self.stack.push(Type::List(result_list));
            }

            // Filtering a list value
            "filter" => {
                let code = self.pop_stack().get_string();
                let vars = self.pop_stack().get_string();
                let list = self.pop_stack().get_list();

                let mut result_list = Vec::new();

                for x in list.iter() {
                    self.memory
                        .entry(vars.clone())
                        .and_modify(|value| *value = x.clone())
                        .or_insert(x.clone());

                    self.evaluate_program(code.clone());
                    if self.pop_stack().get_bool() {
                        result_list.push(x.clone());
                    }
                }

                self.stack.push(Type::List(result_list));
            }

            // Generate value from list
            "reduce" => {
                let code = self.pop_stack().get_string();
                let now = self.pop_stack().get_string();
                let acc = self.pop_stack().get_string();
                let list = self.pop_stack().get_list();

                self.memory
                    .entry(acc.clone())
                    .and_modify(|value| *value = Type::String("".to_string()))
                    .or_insert(Type::String("".to_string()));

                for x in list.iter() {
                    self.memory
                        .entry(now.clone())
                        .and_modify(|value| *value = x.clone())
                        .or_insert(x.clone());

                    self.evaluate_program(code.clone());
                    let result = self.pop_stack();

                    self.memory
                        .entry(acc.clone())
                        .and_modify(|value| *value = result.clone())
                        .or_insert(result);
                }

                let result = self.memory.get(&acc);
                self.stack
                    .push(result.unwrap_or(&Type::String("".to_string())).clone());

                self.memory
                    .entry(acc.clone())
                    .and_modify(|value| *value = Type::String("".to_string()))
                    .or_insert(Type::String("".to_string()));
            }

            // Generate a range
            "range" => {
                let step = self.pop_stack().get_number();
                let max = self.pop_stack().get_number();
                let min = self.pop_stack().get_number();

                let mut range: Vec<Type> = Vec::new();
                let mut i = min;

                while i < max {
                    range.push(Type::Number(i));
                    i += step;
                }

                self.stack.push(Type::List(range));
            }

            // Get length of list
            "len" => {
                let data = self.pop_stack().get_list();
                self.stack.push(Type::Number(data.len() as f64));
            }

            // Commands of memory manage

            // pop in the stack
            "pop" => {
                self.pop_stack();
            }

            // Get size of stack
            "size-stack" => {
                let len: f64 = self.stack.len() as f64;
                self.stack.push(Type::Number(len));
            }

            // Define variable at memory
            "var" => {
                let name = self.pop_stack().get_string();
                let data = self.pop_stack();
                self.memory
                    .entry(name)
                    .and_modify(|value| *value = data.clone())
                    .or_insert(data);
                self.show_variables()
            }

            // Get data type of value
            "type" => {
                let result = match self.pop_stack() {
                    Type::Number(_) => "number".to_string(),
                    Type::String(_) => "string".to_string(),
                    Type::Bool(_) => "bool".to_string(),
                    Type::List(_) => "list".to_string(),
                    Type::Error(_) => "error".to_string(),
                    Type::Object(name, _) => name,
                    Type::Element(_) => "element".to_string(),
                };
                self.stack.push(Type::String(result));
            }

            // Explicit data type casting
            "cast" => {
                let types = self.pop_stack().get_string();
                let mut value = self.pop_stack();
                match types.as_str() {
                    "number" => self.stack.push(Type::Number(value.get_number())),
                    "string" => self.stack.push(Type::String(value.get_string())),
                    "bool" => self.stack.push(Type::Bool(value.get_bool())),
                    "list" => self.stack.push(Type::List(value.get_list())),
                    "error" => self.stack.push(Type::Error(value.get_string())),
                    _ => self.stack.push(value),
                }
            }

            // Is string include only number
            "only-number" => match self.pop_stack().get_string().trim().parse::<f64>() {
                Ok(_) => self.stack.push(Type::Bool(true)),
                Err(_) => self.stack.push(Type::Bool(false)),
            },

            // Get memory information
            "mem" => {
                let mut list: Vec<Type> = Vec::new();
                for (name, _) in self.memory.clone() {
                    list.push(Type::String(name))
                }
                self.stack.push(Type::List(list))
            }

            // Free up memory space of variable
            "free" => {
                let name = self.pop_stack().get_string();
                self.memory.remove(name.as_str());
                self.show_variables();
            }

            // Copy stack's top value
            "copy" => {
                let data = self.pop_stack();
                self.stack.push(data.clone());
                self.stack.push(data);
            }

            // Swap stack's top 2 value
            "swap" => {
                let b = self.pop_stack();
                let a = self.pop_stack();
                self.stack.push(b);
                self.stack.push(a);
            }

            // Commands of object oriented system

            // Generate a instance of object
            "instance" => {
                let data = self.pop_stack().get_list();
                let mut class = self.pop_stack().get_list();
                let mut object: HashMap<String, Type> = HashMap::new();

                let name = if !class.is_empty() {
                    class[0].get_string()
                } else {
                    self.log("Error! the type name is not found.".to_string());
                    self.stack.push(Type::Error("instance-name".to_string()));
                    return;
                };

                let mut index = 0;
                for item in &mut class.to_owned()[1..class.len()].iter() {
                    let mut item = item.to_owned();
                    if item.get_list().len() == 1 {
                        let element = match data.get(index) {
                            Some(value) => value,
                            None => {
                                self.log("Error! initial data is shortage\n".to_string());
                                self.stack
                                    .push(Type::Error("instance-shortage".to_string()));
                                return;
                            }
                        };
                        object.insert(
                            item.get_list()[0].to_owned().get_string(),
                            element.to_owned(),
                        );
                        index += 1;
                    } else if item.get_list().len() >= 2 {
                        let item = item.get_list();
                        object.insert(item[0].clone().get_string(), item[1].clone());
                    } else {
                        self.log("Error! the class data structure is wrong.".to_string());
                        self.stack.push(Type::Error("instance-default".to_string()));
                    }
                }

                self.stack.push(Type::Object(name, object))
            }

            // Get property of object
            "property" => {
                let name = self.pop_stack().get_string();
                let (_, object) = self.pop_stack().get_object();
                self.stack.push(
                    object
                        .get(name.as_str())
                        .unwrap_or(&Type::Error("property".to_string()))
                        .clone(),
                )
            }

            // Call the method of object
            "method" => {
                let method = self.pop_stack().get_string();
                let (name, value) = self.pop_stack().get_object();
                let data = Type::Object(name, value.clone());
                self.memory
                    .entry("self".to_string())
                    .and_modify(|value| *value = data.clone())
                    .or_insert(data);

                let program: String = match value.get(&method) {
                    Some(i) => i.to_owned().get_string().to_string(),
                    None => "".to_string(),
                };

                self.evaluate_program(program);
            }

            // Modify the property of object
            "modify" => {
                let data = self.pop_stack();
                let property = self.pop_stack().get_string();
                let (name, mut value) = self.pop_stack().get_object();
                value
                    .entry(property)
                    .and_modify(|value| *value = data.clone())
                    .or_insert(data.clone());

                self.stack.push(Type::Object(name, value))
            }

            // Get all of properties
            "all" => {
                let (_, value) = self.pop_stack().get_object();
                self.stack.push(Type::List(
                    value
                        .keys()
                        .map(|x| Type::String(x.to_owned()))
                        .collect::<Vec<Type>>(),
                ));
            }

            "get-element-by-id" => {
                let id = self.pop_stack().get_string();
                self.stack.push(get_element_by_id(id));
            }

            "set-inner-html" => {
                let element = self.pop_stack().get_element();
                element.set_inner_html(&self.pop_stack().get_string())
            }

            "get-inner-html" => {
                let element = self.pop_stack().get_element();
                self.stack.push(Type::String(element.inner_html()));
            }

            "get-id" => {
                let element = self.pop_stack().get_element();
                self.stack.push(Type::String(element.id()));
            }

            "set-id" => {
                let element = self.pop_stack().get_element();
                element.set_id(&self.pop_stack().get_string());
            }

            "create-element" => {
                let name = self.pop_stack().get_string();
                let window = web_sys::window().expect("no global `window` exists");
                let document = window.document().expect("should have a document on window");

                self.stack
                    .push(Type::Element(document.create_element(&name).unwrap()));
            }

            "append-child" => {
                let element = self.pop_stack().get_element();
                element
                    .append_child(&self.pop_stack().get_element())
                    .expect("チノちゃん「うるさいですね...」");
            }

            "add-event-listener" => {
                let element = self.pop_stack().get_element();
                element
                    .add_event_listener_with_callback(
                        &self.pop_stack().get_string(),
                        &Function::new_no_args(&format!(
                            "window.run_stack(`{}`)",
                            self.pop_stack().get_string()
                        )),
                    )
                    .expect("チノちゃん「うるさいですね...」");
            }

            // If it is not recognized as a command, use it as a string.
            _ => self.stack.push(Type::String(command)),
        }
    }

    /// Pop stack's top value
    fn pop_stack(&mut self) -> Type {
        if let Some(value) = self.stack.pop() {
            value
        } else {
            self.log(
                "Error! There are not enough values on the stack. returns default value\n"
                    .to_string(),
            );
            Type::String("".to_string())
        }
    }
}
