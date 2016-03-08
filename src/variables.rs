use std::collections::BTreeMap;
use std::io::{stdout, Write};

use super::peg::{Pipeline, Job};
use super::input_editor::readln;
use super::status::{SUCCESS, FAILURE};

pub struct Variables {
    variables: BTreeMap<String, String>,
}

impl Variables {
    pub fn new() -> Variables {
        Variables { variables: BTreeMap::new() }
    }

    pub fn read<I: IntoIterator>(&mut self, args: I) -> i32
        where I::Item: AsRef<str>
    {
        let mut out = stdout();
        for arg in args.into_iter().skip(1) {
            print!("{}=", arg.as_ref().trim());
            if let Err(message) = out.flush() {
                println!("{}: Failed to flush stdout", message);
                return FAILURE;
            }
            if let Some(value) = readln() {
                self.set_var(arg.as_ref(), value.trim());
            }
        }
        SUCCESS
    }

    pub fn let_<I: IntoIterator>(&mut self, args: I) -> i32
        where I::Item: AsRef<str>
    {
        let args = args.into_iter();
        let string: String = args.skip(1).fold(String::new(), |string, x| string + x.as_ref());
        let mut split = string.split('=');
        match (split.next().and_then(|x| if x == "" { None } else { Some(x) }), split.next()) {
            (Some(key), Some(value)) => {
                if !Variables::is_valid_variable_name(&key) {
                    return FAILURE;
                }
                self.variables.insert(key.to_string(), value.to_string());
            },
            (Some(key), None) => {
                self.variables.remove(key);
            },
            _ => {
                for (key, value) in self.variables.iter() {
                    println!("{}={}", key, value);
                }
            }
        }
        SUCCESS
    }

    pub fn set_var(&mut self, name: &str, value: &str) {
        if !name.is_empty() {
            if value.is_empty() {
                self.variables.remove(&name.to_string());
            } else {
                self.variables.insert(name.to_string(), value.to_string());
            }
        }
    }

    pub fn expand_pipeline(&self, pipeline: &Pipeline) -> Pipeline {
        // TODO don't copy everything
        // TODO ugh, I made it worse
        Pipeline::new(
            pipeline.jobs.iter().map(|job| {self.expand_job(job)}).collect(),
            pipeline.stdin_file.clone(),
            pipeline.stdout_file.clone())
    }

    pub fn expand_job(&self, job: &Job) -> Job {
        // TODO don't copy everything
        Job::from_vec_string(job.args
                                .iter()
                                .map(|original: &String| self.expand_string(&original).to_string())
                                .collect(),
                             job.background)
    }

    fn replace_substring(string: &mut String, start: usize, end: usize, replacement: &str) {
        let string_start = string.chars().take(start).collect::<String>();
        let string_end = string.chars().skip(end+1).collect::<String>();
        *string = string_start + replacement + &string_end;
    }

    pub fn is_valid_variable_character(c: char) -> bool {
        c.is_alphanumeric() || c == '_' || c == '?'
    }

    pub fn is_valid_variable_name(name: &str) -> bool {
        name.chars().all(Variables::is_valid_variable_character)
    }

    #[inline]
    pub fn expand_string<'a>(&'a self, original: &'a str) -> String {
        let mut new = original.to_owned();
        let mut replacements: Vec<(usize, usize, String)> = vec![];
        for (n, _) in original.match_indices("$") {
            if n > 0 {
                if let Some(c) = original.chars().nth(n-1) {
                    if c == '\\' {
                        continue;
                    }
                }
            }
            let mut var_name = "".to_owned();
            for (i, c) in original.char_indices().skip(n+1) { // skip the dollar sign
                if Variables::is_valid_variable_character(c) {
                    var_name.push(c);
                    if i == original.len() - 1 {
                        replacements.push((n, i, var_name.clone()));
                        break;
                    }
                } else {
                    replacements.push((n, i-1, var_name.clone()));
                    break;
                }
            }
        }

        for &(start, end, ref var_name) in replacements.iter().rev() {
            let value: &str = match self.variables.get(var_name) {
                Some(v) => &v,
                None => ""
            };
            Variables::replace_substring(&mut new, start, end, value);
        }
        new.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use status::FAILURE;

    #[test]
    fn undefined_variable_expands_to_empty_string() {
        let variables = Variables::new();
        let expanded = variables.expand_string("$FOO");
        assert_eq!("", &expanded);
    }

    #[test]
    fn let_and_expand_a_variable() {
        let mut variables = Variables::new();
        variables.let_(vec!["let", "FOO", "=", "BAR"]);
        let expanded = variables.expand_string("$FOO");
        assert_eq!("BAR", &expanded);
    }

    #[test]
    fn set_var_and_expand_a_variable() {
        let mut variables = Variables::new();
        variables.set_var("FOO", "BAR");
        let expanded = variables.expand_string("$FOO");
        assert_eq!("BAR", &expanded);
    }

    #[test]
    fn remove_a_variable_with_let() {
        let mut variables = Variables::new();
        variables.let_(vec!["let", "FOO", "=", "BAR"]);
        variables.let_(vec!["let", "FOO"]);
        let expanded = variables.expand_string("$FOO");
        assert_eq!("", &expanded);
    }

    #[test]
    fn expand_several_variables() {
        let mut variables = Variables::new();
        variables.let_(vec!["let", "FOO", "=", "BAR"]);
        variables.let_(vec!["let", "X", "=", "Y"]);
        let expanded = variables.expand_string("variables: $FOO $X");
        assert_eq!("variables: BAR Y", &expanded);
    }

    #[test]
    fn replace_substring() {
        let mut string = "variable: $FOO".to_owned();
        Variables::replace_substring(&mut string, 10, 13, "BAR");
        assert_eq!("variable: BAR", string);
    }

    #[test]
    fn escape_with_backslash() {
        let variables = Variables::new();
        let expanded = variables.expand_string("\\$FOO");
        assert_eq!("\\$FOO", &expanded);
    }

    #[test]
    fn let_checks_variable_name() {
        let mut variables = Variables::new();
        let return_status = variables.let_(vec!["let", ",;!:", "=", "FOO"]);
        assert_eq!(FAILURE, return_status);
    }
}
