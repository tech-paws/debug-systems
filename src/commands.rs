use regex::Regex;

use std::collections::HashMap;
use std::sync::MutexGuard;

use crate::state::{DebugState, DEBUG_STATE};

pub const COMMANDS_HISTORY_CAPACITY: usize = 100;

pub struct CommandsState {
    pub history: Vec<String>,
    pub registry: Vec<CommandRegistryEntry>,
    pub index: HashMap<String, Command>,
}

impl Default for CommandsState {
    fn default() -> Self {
        CommandsState {
            history: Vec::with_capacity(COMMANDS_HISTORY_CAPACITY),
            registry: Vec::new(),
            index: HashMap::new(),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum CommandArgument {
    Number(f64),
    String(String),
    Bool(bool),
}

#[derive(PartialEq, Debug)]
pub enum Token<'a> {
    Number(f64),
    Id(&'a str),
    String(&'a str),
    Bool(bool),
}

pub struct Command {
    pub namespace: String,
    pub name: String,
    pub executor: fn(&mut MutexGuard<DebugState>, &[CommandArgument]) -> Result<(), String>,
}

#[derive(PartialEq, Debug)]
pub struct CommandRequest {
    pub command: String,
    pub arguments: Vec<CommandArgument>,
}

pub struct CommandRegistryEntry {
    pub namespace: String,
    pub name: String,
    pub args: String,
    pub _desc: &'static str,
}

pub fn register_command(
    debug_state: &mut MutexGuard<DebugState>,
    desc: &'static str,
    command: Command,
) {
    debug_state.commands.registry.push(CommandRegistryEntry {
        namespace: command.namespace.clone(),
        name: command.name.clone(),
        args: String::from("<arguments: int>"),
        _desc: desc,
    });

    debug_state.commands.index.insert(
        format!("{}::{}", &command.namespace, &command.name),
        command,
    );
}

pub fn execute_command(command: &str) -> Result<(), String> {
    let debug_state = &mut DEBUG_STATE.lock().expect("failed to get debug state");
    debug_state.commands.history.push(String::from(command));
    let request = parse_command(command)?;
    execute_command_request(debug_state, &request)
}

fn parse_command(command: &str) -> Result<CommandRequest, String> {
    let tokens = tokenize(command);

    if tokens.is_empty() {
        Err(String::from("Command can't be empty"))
    } else {
        let command = if let Token::Id(id) = tokens[0] {
            id
        } else {
            return Err(String::from("Parse error"));
        };

        let command = String::from(command);
        let mut arguments = Vec::new();

        for token in tokens.iter().skip(1) {
            match *token {
                Token::String(value) => {
                    arguments.push(CommandArgument::String(String::from(value)))
                }
                Token::Number(value) => arguments.push(CommandArgument::Number(value)),
                Token::Bool(value) => arguments.push(CommandArgument::Bool(value)),
                _ => return Err(String::from("Parse error")),
            }
        }

        Ok(CommandRequest { command, arguments })
    }
}

fn tokenize(command: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();

    let re = Regex::new(r###"(?P<bool>true|false)|("(?P<string>[^"]*)")|(?P<id>[a-zA-Z_][a-zA-Z:0-9_-]+)|(?P<number>[0-9]+(\.[0-9]+)?)"###).unwrap();

    for cap in re.captures_iter(command) {
        if let Some(m) = cap.name("id") {
            tokens.push(Token::Id(m.as_str()));
        } else if let Some(m) = cap.name("string") {
            tokens.push(Token::String(m.as_str()));
        } else if let Some(m) = cap.name("number") {
            tokens.push(Token::Number(m.as_str().parse().unwrap()));
        } else if let Some(m) = cap.name("bool") {
            tokens.push(Token::Bool(m.as_str().parse().unwrap()));
        }
    }

    tokens
}

fn execute_command_request(
    debug_state: &mut MutexGuard<DebugState>,
    request: &CommandRequest,
) -> Result<(), String> {
    match debug_state.commands.index.get(&request.command) {
        Some(command) => {
            let executor = command.executor;
            executor(debug_state, &request.arguments)
        }
        None => Err(format!("Command '{}' not found", request.command)),
    }
}

pub fn require(cond: bool, msg: &str) -> Result<(), String> {
    if cond {
        Ok(())
    } else {
        Err(String::from(msg))
    }
}

#[cfg(test)]
mod tests {
    use crate::commands;
    use std::sync::MutexGuard;

    #[test]
    fn tokenize() {
        let tokens = commands::tokenize("greet::hello test 12 55.9 \"Hello World!\" false true");
        assert_eq!(
            tokens,
            vec![
                commands::Token::Id("greet::hello"),
                commands::Token::Id("test"),
                commands::Token::Number(12.0),
                commands::Token::Number(55.9),
                commands::Token::String("Hello World!"),
                commands::Token::Bool(false),
                commands::Token::Bool(true),
            ]
        )
    }

    #[test]
    fn parse_command_without_arguments() {
        let request = commands::parse_command("greet::say_hello").unwrap();
        assert_eq!(
            request,
            commands::CommandRequest {
                command: String::from("greet::say_hello"),
                arguments: vec![],
            }
        );
    }

    #[test]
    fn parse_command_number() {
        let request = commands::parse_command("math::max 12 88.12").unwrap();
        assert_eq!(
            request,
            commands::CommandRequest {
                command: String::from("math::max"),
                arguments: vec![
                    commands::CommandArgument::Number(12.0),
                    commands::CommandArgument::Number(88.12)
                ],
            }
        );
    }

    #[test]
    fn parse_command_string() {
        let request = commands::parse_command("greet::print_hello \"<User Name>\"").unwrap();
        assert_eq!(
            request,
            commands::CommandRequest {
                command: String::from("greet::print_hello"),
                arguments: vec![commands::CommandArgument::String(String::from(
                    "<User Name>"
                ))],
            }
        );
    }

    #[test]
    fn parse_command() {
        let request =
            commands::parse_command("math::max_and_print 12 88.12 \"Max: \" true").unwrap();

        assert_eq!(
            request,
            commands::CommandRequest {
                command: String::from("math::max_and_print"),
                arguments: vec![
                    commands::CommandArgument::Number(12.0),
                    commands::CommandArgument::Number(88.12),
                    commands::CommandArgument::String(String::from("Max: ")),
                    commands::CommandArgument::Bool(true)
                ],
            }
        );
    }

    #[test]
    fn execute_command() {
        {
            let debug_state = &mut commands::DEBUG_STATE
                .lock()
                .expect("failed to get debug state");

            commands::register_command(
                debug_state,
                "Test commands",
                commands::Command {
                    namespace: String::from("math"),
                    name: String::from("sum"),
                    executor: sum_command,
                },
            );
        }
        assert_eq!(true, commands::execute_command("math::sum 2 2").is_ok());
    }

    #[test]
    fn execute_command_failed_type() {
        {
            let debug_state = &mut commands::DEBUG_STATE
                .lock()
                .expect("failed to get debug state");

            commands::register_command(
                debug_state,
                "Test commands",
                commands::Command {
                    namespace: String::from("math"),
                    name: String::from("sum"),
                    executor: sum_command,
                },
            );
        }

        let res = commands::execute_command("math::sum 2 \"2\"");

        assert_eq!(true, res.is_err());
        assert_eq!("second argument should be number", res.err().unwrap());
    }

    #[test]
    fn execute_command_failed() {
        {
            let debug_state = &mut commands::DEBUG_STATE
                .lock()
                .expect("failed to get debug state");

            commands::register_command(
                debug_state,
                "Test commands",
                commands::Command {
                    namespace: String::from("math"),
                    name: String::from("sum"),
                    executor: sum_command,
                },
            );
        }

        let res = commands::execute_command("math::sum 2");

        assert_eq!(true, res.is_err());
        assert_eq!("bad arguments length", res.err().unwrap());
    }

    fn sum_command(
        _: &mut MutexGuard<commands::DebugState>,
        arguments: &[commands::CommandArgument],
    ) -> Result<(), String> {
        commands::require(arguments.len() == 2, "bad arguments length")?;

        let a = match arguments[0] {
            commands::CommandArgument::Number(val) => Ok(val),
            _ => Err(String::from("first argument should be number")),
        }?;

        let b = match arguments[1] {
            commands::CommandArgument::Number(val) => Ok(val),
            _ => Err(String::from("second argument should be number")),
        }?;

        println!("{} + {} = {}", a, b, a + b);
        Ok(())
    }
}
