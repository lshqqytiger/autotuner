use std::{error, ffi::OsStr, fmt, io::Read, path::Path, process::Command};

#[derive(Debug)]
pub(crate) enum Error {
    Spawn,
    Compilation(Option<String>),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Spawn => write!(f, "Failed to spawn compiler process"),
            Error::Compilation(Some(output)) => {
                write!(f, "Compilation failed:\n{}", output)
            }
            Error::Compilation(None) => write!(f, "Compilation failed"),
        }
    }
}

pub(crate) fn compile<S: AsRef<OsStr>>(
    compiler: &String,
    output: &Path,
    arguments: impl Iterator<Item = S>,
) -> Result<(), Error> {
    let mut compiler = Command::new(compiler);
    let compiler = compiler
        .arg("-shared")
        .arg("-o")
        .arg(output)
        .args(arguments);

    let mut compiler = compiler.spawn().map_err(|_| Error::Spawn)?;
    let status = compiler.wait().map_err(|_| Error::Spawn)?;

    if status.success() {
        Ok(())
    } else {
        let reason = compiler
            .stderr
            .take()
            .map(|mut stderr| {
                let mut buffer = String::new();
                if stderr.read_to_string(&mut buffer).is_ok() {
                    Some(buffer)
                } else {
                    None
                }
            })
            .flatten();
        Err(Error::Compilation(reason))
    }
}
