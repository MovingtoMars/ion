use shell::directory_stack::DirectoryStack;
use shell::variables::Variables;
use types::Value;
use parser::{expand_string, ExpanderFunctions, Select};

#[derive(Debug, PartialEq)]
pub enum ForExpression {
    Multiple(Vec<Value>),
    Normal(Value),
    Range(usize, usize)
}

impl ForExpression {
    pub fn new(expression: &[String], dir_stack: &DirectoryStack, variables: &Variables) -> ForExpression {
        let output: Vec<_> = expression.iter()
            .flat_map(|expression| expand_string(expression, &get_expanders!(variables, dir_stack), true))
            .collect();

        if output.len() == 1 {
            let output = output.into_iter().next().unwrap();
            {
                let mut bytes_iterator = output.bytes().enumerate();
                while let Some((id, byte)) = bytes_iterator.next() {
                    match byte {
                        b'0'...b'9' => continue,
                        b'.' => match output[0..id].parse::<usize>().ok() {
                            Some(first_number) => {
                                let mut dots = 1;
                                for (_, byte) in bytes_iterator {
                                    if byte == b'.' { dots += 1 } else { break }
                                }

                                match output[id+dots..].parse::<usize>().ok() {
                                    Some(second_number) => {
                                        match dots {
                                            2 => return ForExpression::Range(first_number, second_number),
                                            3 => return ForExpression::Range(first_number, second_number+1),
                                            _ => break
                                        }
                                    },
                                    None => break
                                }
                            },
                            None => break
                        },
                        _ => break
                    }
                }
            }
            ForExpression::Normal(output)
        } else {
            ForExpression::Multiple(output)
        }
    }
}

#[test]
fn for_inclusive_range() {
    let dir_stack = DirectoryStack::new().unwrap();
    let variables = Variables::default();
    let input = &["1...10".to_owned()];
    assert_eq!(ForExpression::new(input, &dir_stack, &variables), ForExpression::Range(1, 11));
}

#[test]
fn for_exclusive_range() {
    let dir_stack = DirectoryStack::new().unwrap();
    let variables = Variables::default();
    let input = &["1..10".to_owned()];
    assert_eq!(ForExpression::new(input, &dir_stack, &variables), ForExpression::Range(1, 10));
}

#[test]
fn for_normal() {
    let dir_stack = DirectoryStack::new().unwrap();
    let variables = Variables::default();
    let output = vec!["1".to_owned(), "2".to_owned(), "3".to_owned(), "4".to_owned(), "5".to_owned()];
    assert_eq!(ForExpression::new(&output.clone(), &dir_stack, &variables), ForExpression::Multiple(output));
}

#[test]
fn for_variable() {
    let dir_stack = DirectoryStack::new().unwrap();
    let mut variables = Variables::default();
    variables.set_var("A", "1 2 3 4 5");
    assert_eq!(ForExpression::new(&["$A".to_owned()], &dir_stack, &variables),
        ForExpression::Normal("1 2 3 4 5".to_owned()));
}
