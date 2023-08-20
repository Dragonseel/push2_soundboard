use crate::MyError;

use super::ActionState;

#[derive(Deserialize)]
pub struct SingleCommandConfig {}

pub struct Command {
    command: String,
    args: Vec<String>,
    last_executed: Option<std::time::Instant>,
}

impl Command {
    pub fn new(command: String, args: Vec<String>) -> Command {
        Command {
            command,
            args,
            last_executed: None,
        }
    }

    pub fn execute(&mut self) -> Result<ActionState, MyError> {
        if self.last_executed.is_some() {
            return Ok(ActionState::Playing);
        }

        let _child = std::process::Command::new(self.command.clone())
            .args(self.args.clone())
            .spawn()
            .expect("Could not execute command.");

        self.last_executed = Some(std::time::Instant::now());

        Ok(ActionState::Started)
    }

    pub fn update(&mut self) -> Result<ActionState, MyError> {
        if let Some(last_executed) = self.last_executed {
            let duration = std::time::Instant::now() - last_executed;

            if duration > std::time::Duration::from_secs(1) {
                self.last_executed = None;
                return Ok(ActionState::Stopped);
            } else {
                return Ok(ActionState::Playing);
            }
        } else {
            return Ok(ActionState::None);
        }
    }

    pub fn is_running(&self) -> ActionState {
        if let Some(last_executed) = self.last_executed {
            let duration = std::time::Instant::now() - last_executed;

            if duration > std::time::Duration::from_secs(1) {
                return ActionState::Stopped;
            } else {
                return ActionState::Playing;
            }
        } else {
            return ActionState::None;
        }
    }
}
