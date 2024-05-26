pub struct Command {
    pub name: String,
    pub aliases: Vec<String>,
    pub help: String,
    pub cmd: Box<dyn Fn(&dyn Iterator<Item = &str>) -> String>,
}

impl Command {
    pub fn new(
        name: String,
        aliases: Vec<String>,
        help: String,
        cmd: Box<dyn Fn(&dyn Iterator<Item = &str>) -> String>,
    ) -> Self {
        Self {
            name,
            aliases,
            help,
            cmd,
        }
    }
}
