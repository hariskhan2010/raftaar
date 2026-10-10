use std::sync::Mutex;

use crate::Result;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    pub fn ok(stdout: &str) -> Self {
        Output { status: 0, stdout: stdout.into(), stderr: String::new() }
    }

    pub fn fail(stderr: &str) -> Self {
        Output { status: 1, stdout: String::new(), stderr: stderr.into() }
    }

    pub fn success(&self) -> bool {
        self.status == 0
    }
}

pub trait Shell: Send + Sync {
    fn run(&self, prog: &str, args: &[&str]) -> Result<Output>;
}

/// Runs real processes without flashing a console window.
pub struct RealShell;

impl Shell for RealShell {
    fn run(&self, prog: &str, args: &[&str]) -> Result<Output> {
        let mut cmd = std::process::Command::new(prog);
        cmd.args(args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let out = cmd.output()?;
        Ok(Output {
            status: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }
}

/// Scripted shell for tests. The first rule whose needle appears in the
/// command line answers; unmatched commands succeed with empty output.
#[derive(Default)]
pub struct MockShell {
    rules: Mutex<Vec<(String, Output)>>,
    pub calls: Mutex<Vec<String>>,
}

impl MockShell {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on(self, needle: &str, out: Output) -> Self {
        self.rules.lock().unwrap().push((needle.into(), out));
        self
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl Shell for MockShell {
    fn run(&self, prog: &str, args: &[&str]) -> Result<Output> {
        let line = std::iter::once(prog).chain(args.iter().copied()).collect::<Vec<_>>().join(" ");
        self.calls.lock().unwrap().push(line.clone());
        let rules = self.rules.lock().unwrap();
        Ok(rules
            .iter()
            .find(|(needle, _)| line.contains(needle.as_str()))
            .map(|(_, out)| out.clone())
            .unwrap_or_default())
    }
}
