//! [`OvmProcess`]: OpenVM proofs verified by `willow-logs-verify-openvm serve`, a separate
//! binary, since OpenVM's verifier cannot be linked with SP1's (Cargo.toml). It speaks the line
//! protocol in `zkvm/logset-guest-openvm/verify/src/serve.rs` and verifies under its own pinned
//! verifying keys, the minted programs only; it is started on the first OpenVM proof.

use logset_query::ProofCheck;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use willow_logs_verify_openvm::pinned::{self, Kind};

/// The OpenVM verifier's binary name: looked for next to this executable, then on `PATH`.
pub const OPENVM_VERIFIER: &str = "willow-logs-verify-openvm";

/// A [`ProofCheck`] for OpenVM proofs, run in `willow-logs-verify-openvm serve`.
pub struct OvmProcess {
    program: PathBuf,
    state: State,
}

enum State {
    Idle,
    Running(Running),
    Failed(String),
}

struct Running {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl Drop for Running {
    fn drop(&mut self) {
        // End of input ends `serve`; kill it only if it does not stop.
        drop(self.stdin.take());
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if !matches!(self.child.try_wait(), Ok(Some(_))) {
                let _ = self.child.kill();
            }
        }
        let _ = self.child.wait();
    }
}

impl OvmProcess {
    /// `program`, else [`OPENVM_VERIFIER`] next to this executable, else on `PATH`.
    pub fn new(program: Option<PathBuf>) -> Self {
        let program = program.unwrap_or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| Some(p.parent()?.join(OPENVM_VERIFIER)))
                .filter(|p| p.is_file())
                .unwrap_or_else(|| PathBuf::from(OPENVM_VERIFIER))
        });
        Self {
            program,
            state: State::Idle,
        }
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    fn running(&mut self) -> Result<&mut Running, String> {
        if let State::Idle = self.state {
            self.state = match self.start() {
                Ok(r) => State::Running(r),
                Err(e) => State::Failed(e),
            };
        }
        match &mut self.state {
            State::Running(r) => Ok(r),
            State::Failed(e) => Err(e.clone()),
            State::Idle => unreachable!(),
        }
    }

    fn start(&self) -> Result<Running, String> {
        let shown = self.program.display();
        let mut child = Command::new(&self.program)
            .arg("serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| {
                format!(
                    "no OpenVM verifier ({shown}: {e}); put {OPENVM_VERIFIER} next to this \
                     binary or pass --openvm-verifier <path>"
                )
            })?;
        let stdin = child.stdin.take();
        let stdout = BufReader::new(child.stdout.take().expect("piped"));
        let mut r = Running {
            child,
            stdin,
            stdout,
        };
        let line = r
            .line()
            .map_err(|e| format!("OpenVM verifier {shown}: {e}"))?;
        // It must verify under exactly the keys this build trusts OpenVM parts with.
        let want: Vec<String> = Kind::ALL
            .iter()
            .map(|k| pinned::key(*k).map(hex::encode))
            .collect::<Result<_, _>>()?;
        match line.split_once(' ') {
            Some(("ready", keys)) if keys.split(' ').eq(want.iter().map(String::as_str)) => Ok(r),
            Some(("ready", keys)) => Err(format!(
                "OpenVM verifier {shown} trusts other programs ({keys}) than this build \
                 ({}); use the {OPENVM_VERIFIER} built with it",
                want.join(" ")
            )),
            Some(("error", why)) => Err(format!("OpenVM verifier {shown}: {why}")),
            _ => Err(format!("OpenVM verifier {shown} said {line:?}")),
        }
    }
}

impl Running {
    fn line(&mut self) -> Result<String, String> {
        let mut line = String::new();
        match self.stdout.read_line(&mut line) {
            Ok(0) => Err("stopped".into()),
            Ok(_) => Ok(line.trim_end().to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn ask(
        &mut self,
        vkey: &[u8; 32],
        proof: &[u8],
        pv: &[u8],
    ) -> Result<Result<(), String>, String> {
        let stdin = self.stdin.as_mut().ok_or("stopped")?;
        writeln!(
            stdin,
            "{} {} {}",
            hex::encode(vkey),
            hex::encode(pv),
            hex::encode(proof)
        )
        .and_then(|_| stdin.flush())
        .map_err(|e| e.to_string())?;
        let line = self.line()?;
        match line.split_once(' ').unwrap_or((&line, "")) {
            ("ok", "") => Ok(Ok(())),
            ("fail", why) => Ok(Err(why.to_string())),
            _ => Err(format!("said {line:?}")),
        }
    }
}

impl ProofCheck for OvmProcess {
    fn check(&mut self, vkey: &[u8; 32], proof: &[u8], public_values: &[u8]) -> Result<(), String> {
        if logset_query::ProofSystem::of(proof) != Some(logset_query::ProofSystem::OpenVm) {
            return Err("not an OpenVM proof".into());
        }
        let shown = self.program.display().to_string();
        let r = self.running()?;
        match r.ask(vkey, proof, public_values) {
            Ok(verdict) => verdict,
            Err(e) => {
                let e = format!("OpenVM verifier {shown}: {e}");
                self.state = State::Failed(e.clone());
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ovm(bytes: &[u8]) -> Vec<u8> {
        [&logset_query::OPENVM_PROOF_MAGIC[..], bytes].concat()
    }

    #[test]
    fn a_missing_verifier_fails_every_openvm_check_and_no_sp1_one_reaches_it() {
        let mut c = OvmProcess::new(Some("/nonexistent/willow-logs-verify-openvm".into()));
        assert_eq!(
            c.check(&[0; 32], &[1, 0, 0, 0], b"").unwrap_err(),
            "not an OpenVM proof"
        );
        let e = c.check(&[0; 32], &ovm(b"x"), b"").unwrap_err();
        assert!(e.starts_with("no OpenVM verifier (/nonexistent/"), "{e}");
        assert_eq!(c.check(&[0; 32], &ovm(b"x"), b"").unwrap_err(), e);
    }

    /// A stand-in verifier from a shell script: what it says decides the verdict.
    fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let p = dir.join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p
    }

    #[test]
    fn the_verifier_must_trust_exactly_the_minted_programs_and_answer_each_line() {
        let dir = std::env::temp_dir().join(format!("ovm-process-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let keys: Vec<String> = Kind::ALL
            .iter()
            .map(|k| hex::encode(pinned::key(*k).unwrap()))
            .collect();
        let ready = format!("ready {}", keys.join(" "));
        // Answers ok, then fail, then stops.
        let good = script(
            &dir,
            "good",
            &format!("echo '{ready}'; read a; echo ok; read b; echo 'fail no'; read c"),
        );
        let mut c = OvmProcess::new(Some(good));
        c.check(&[0; 32], &ovm(b"x"), b"").unwrap();
        assert_eq!(c.check(&[0; 32], &ovm(b"x"), b"").unwrap_err(), "no");
        let e = c.check(&[0; 32], &ovm(b"x"), b"").unwrap_err();
        assert!(e.ends_with("stopped"), "{e}");
        // Other keys, an error, or nonsense: refused before any proof.
        let other = script(
            &dir,
            "other",
            &format!("echo 'ready {} {} {}'", "00".repeat(32), keys[1], keys[2]),
        );
        let e = OvmProcess::new(Some(other))
            .check(&[0; 32], &ovm(b"x"), b"")
            .unwrap_err();
        assert!(e.contains("trusts other programs"), "{e}");
        let err = script(
            &dir,
            "err",
            "echo 'error pins/vk/agg.vk does not decode'; exit 1",
        );
        let e = OvmProcess::new(Some(err))
            .check(&[0; 32], &ovm(b"x"), b"")
            .unwrap_err();
        assert!(e.ends_with(": pins/vk/agg.vk does not decode"), "{e}");
        let odd = script(&dir, "odd", &format!("echo '{ready}'; read a; echo yes"));
        let e = OvmProcess::new(Some(odd))
            .check(&[0; 32], &ovm(b"x"), b"")
            .unwrap_err();
        assert!(e.ends_with("said \"yes\""), "{e}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
