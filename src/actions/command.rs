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

    pub fn execute(&mut self) -> bool {
        println!("Testing");

        let output = std::process::Command::new(self.command.clone())
            .args(self.args.clone())
            .output()
            .expect("Did not execute thingy.");

        self.last_executed = Some(std::time::Instant::now());

        println!(
            "Output: {:?}",
            String::from_utf8(output.stdout).unwrap().trim_end()
        );

        true
    }

    pub fn update(&mut self) -> bool {
        if self.last_executed.is_none() {
            return false;
        }

        let duration = std::time::Instant::now() - self.last_executed.unwrap();

        if duration > std::time::Duration::from_secs(1) {
            self.last_executed = None;
            return false;
        } else {
            return true;
        }
    }

    pub fn is_running(&self) -> bool {
        if self.last_executed.is_none() {
            return false;
        }

        let duration = std::time::Instant::now() - self.last_executed.unwrap();

        if duration > std::time::Duration::from_secs(1) {
            return false;
        } else {
            return true;
        }
    }
}
