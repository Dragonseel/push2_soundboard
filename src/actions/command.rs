#[derive(Deserialize)]
pub struct SingleCommandConfig {}

pub struct Command {
    command: String,
    args: Vec<String>,
}

impl Command {
    pub fn new(command: String, args: Vec<String>) -> Command {
        Command { command, args }
    }

    pub fn execute(&mut self) -> bool {

        println!("Testing");

        let output = std::process::Command::new(self.command.clone())
            .args(self.args.clone())
            .output()
            .expect("Did not execute thingy.");

        println!("Output: {:?}", String::from_utf8(output.stdout).unwrap());
        
        true
    }
}
