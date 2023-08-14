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

    pub fn execute(&mut self) -> ActionState {
        if self.last_executed.is_some() {
            return ActionState::Playing;
        }

        let _child = std::process::Command::new(self.command.clone())
            .args(self.args.clone()).spawn().expect("Could not execute command.");

        self.last_executed = Some(std::time::Instant::now());

        ActionState::Started
    }

    pub fn update(&mut self) -> ActionState {
        if self.last_executed.is_none() {
            return ActionState::None;
        }

        let duration = std::time::Instant::now() - self.last_executed.unwrap();

        if duration > std::time::Duration::from_secs(1) {
            self.last_executed = None;
            return ActionState::Stopped;
        } else {
            return ActionState::Playing;
        }
    }

    pub fn is_running(&self) -> ActionState {
        if self.last_executed.is_none() {
            return ActionState::None;
        }

        let duration = std::time::Instant::now() - self.last_executed.unwrap();

        if duration > std::time::Duration::from_secs(1) {
            return ActionState::Stopped;
        } else {
            return ActionState::Playing;
        }
    }
}
