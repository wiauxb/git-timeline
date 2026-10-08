//! The only place that spawns `git`.

pub mod blame;
pub mod blob;
pub mod log;
pub mod repo;
pub mod show;
pub mod status;

use std::ffi::{OsStr, OsString};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::error::{Error, Result};

#[derive(Clone, Debug)]
pub struct Git {
    dir: PathBuf,
    env: Arc<Vec<(OsString, OsString)>>,
}

impl Git {
    /// Every command runs with `-C <dir>`.
    pub fn new(dir: PathBuf) -> Self {
        Git { dir, env: Arc::new(Vec::new()) }
    }

    /// Extra environment for every command (test fixtures).
    #[doc(hidden)]
    pub fn with_env(dir: PathBuf, env: Vec<(OsString, OsString)>) -> Self {
        Git { dir, env: Arc::new(env) }
    }

    /// Same settings, other directory.
    pub fn at(&self, dir: PathBuf) -> Self {
        Git { dir, env: self.env.clone() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn command<I, S>(&self, args: I) -> Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut c = Command::new("git");
        c.arg("-C")
            .arg(&self.dir)
            .args(["--no-pager", "-c", "color.ui=never", "-c", "core.quotepath=off"])
            .args(["-c", "log.showSignature=false"])
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("LC_ALL", "C")
            .env_remove("GIT_PAGER")
            .env_remove("PAGER")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in self.env.iter() {
            c.env(k, v);
        }
        c
    }

    pub fn output<S: AsRef<OsStr>>(&self, args: &[S]) -> Result<Vec<u8>> {
        let out = self.command(args).output().map_err(spawn_error)?;
        if out.status.success() {
            Ok(out.stdout)
        } else {
            Err(git_error(args, out.status.code(), &out.stderr))
        }
    }

    /// Like `output`, but a non-zero exit returns `Ok(None)` (for `--verify -q` style probes).
    pub fn probe<S: AsRef<OsStr>>(&self, args: &[S]) -> Result<Option<Vec<u8>>> {
        let out = self.command(args).output().map_err(spawn_error)?;
        Ok(out.status.success().then_some(out.stdout))
    }

    pub fn output_with_stdin<S: AsRef<OsStr>>(&self, args: &[S], input: &[u8]) -> Result<Vec<u8>> {
        let mut child = self.command(args).stdin(Stdio::piped()).spawn().map_err(spawn_error)?;
        let mut stdin = child.stdin.take().expect("piped stdin");
        let input = input.to_vec();
        let writer = std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
        let out = child.wait_with_output()?;
        let _ = writer.join();
        if out.status.success() {
            Ok(out.stdout)
        } else {
            Err(git_error(args, out.status.code(), &out.stderr))
        }
    }

    /// Spawn and return a reader on stdout for incremental parsing.
    pub fn stream<S: AsRef<OsStr>>(&self, args: &[S]) -> Result<GitStream> {
        let mut child = self.command(args).spawn().map_err(spawn_error)?;
        let stdout = BufReader::with_capacity(64 * 1024, child.stdout.take().expect("piped stdout"));
        let mut stderr_pipe = child.stderr.take().expect("piped stderr");
        let stderr = Arc::new(Mutex::new(Vec::new()));
        let sink = stderr.clone();
        let stderr_thread = std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = stderr_pipe.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let mut s = sink.lock().unwrap();
                if s.len() < 64 * 1024 {
                    s.extend_from_slice(&buf[..n]);
                }
            }
        });
        Ok(GitStream {
            child: Some(child),
            stdout,
            stderr,
            stderr_thread: Some(stderr_thread),
            args: join_args(args),
        })
    }
}

pub struct GitStream {
    child: Option<Child>,
    pub stdout: BufReader<ChildStdout>,
    stderr: Arc<Mutex<Vec<u8>>>,
    stderr_thread: Option<JoinHandle<()>>,
    args: String,
}

impl GitStream {
    pub fn finish(mut self) -> Result<()> {
        let mut child = self.child.take().expect("child present until finish");
        let status = child.wait()?;
        if let Some(t) = self.stderr_thread.take() {
            let _ = t.join();
        }
        if status.success() {
            Ok(())
        } else {
            let stderr = self.stderr.lock().unwrap();
            Err(Error::Git {
                args: self.args.clone(),
                status: status.code(),
                stderr: String::from_utf8_lossy(&stderr).trim().to_string(),
            })
        }
    }
}

impl Drop for GitStream {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

fn spawn_error(e: std::io::Error) -> Error {
    if e.kind() == std::io::ErrorKind::NotFound {
        Error::GitNotFound
    } else {
        Error::Io(e)
    }
}

fn join_args<S: AsRef<OsStr>>(args: &[S]) -> String {
    args.iter()
        .map(|a| a.as_ref().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

fn git_error<S: AsRef<OsStr>>(args: &[S], status: Option<i32>, stderr: &[u8]) -> Error {
    Error::Git {
        args: join_args(args),
        status,
        stderr: String::from_utf8_lossy(stderr).trim().to_string(),
    }
}
