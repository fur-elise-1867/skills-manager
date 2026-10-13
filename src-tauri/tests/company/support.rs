use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
};

pub struct MockServer {
    child: Child,
    pub url: String,
}
impl MockServer {
    pub fn start(scenario: &str) -> Self {
        Self::start_with_revision(scenario, None)
    }
    pub fn start_with_revision(scenario: &str, revision: Option<&std::path::Path>) -> Self {
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("scripts/company-mock-store.mjs");
        let mut command = Command::new("node");
        command
            .arg(script)
            .args(["--port", "0", "--scenario", scenario])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        if let Some(path) = revision {
            command.arg("--revision-file").arg(path);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command.spawn().expect("start mock HTTP Store");
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let url = format!(
            "http://{}",
            line.split("http://")
                .nth(1)
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap()
        );
        Self { child, url }
    }
    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Drop for MockServer {
    fn drop(&mut self) {
        self.stop();
    }
}
