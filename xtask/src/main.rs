use anyhow::{bail, Result};
use structopt::StructOpt;

use std::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, StructOpt)]
enum XTask {
    Ghp,
}

fn copy<U: AsRef<Path>, V: AsRef<Path>>(from: U, to: V) -> Result<()> {
    let mut stack = Vec::new();
    stack.push(PathBuf::from(from.as_ref()));

    let output_root = PathBuf::from(to.as_ref());
    let input_root = PathBuf::from(from.as_ref()).components().count();

    while let Some(working_path) = stack.pop() {
        println!("process: {:?}", &working_path);

        // Generate a relative path
        let src: PathBuf = working_path.components().skip(input_root).collect();

        // Create a destination if missing
        let dest = if src.components().count() == 0 {
            output_root.clone()
        } else {
            output_root.join(&src)
        };
        if fs::metadata(&dest).is_err() {
            println!("mkdir: {:?}", dest);
            fs::create_dir_all(&dest)?;
        }

        for entry in fs::read_dir(working_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                match path.file_name() {
                    Some(filename) => {
                        let dest_path = dest.join(filename);
                        println!("copy: {:?} -> {:?}", &path, &dest_path);
                        fs::copy(&path, &dest_path)?;
                    }
                    None => {
                        println!("failed: {:?}", path);
                    }
                }
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = XTask::from_args();
    match args {
        XTask::Ghp => {
            let cargo = env::var("CARGO")
                .map(PathBuf::from)
                .ok()
                .unwrap_or_else(|| PathBuf::from("cargo"));
            let status = Command::new(cargo).arg("doc").arg("--no-deps").status()?;
            if !status.success() {
                bail!("The 'cargo doc' command failed");
            }
            let status = Command::new("git").arg("checkout").arg("gh-pages").status()?;
            if !status.success() {
                bail!("The 'git checkout gh-pages' command failed");
            }
            let mut target_doc_dir = PathBuf::from("target");
            target_doc_dir.push("doc");
            copy(target_doc_dir, env::current_dir()?)?;
            // TODO: Add committing changes
            // TODO: Add pushing changes to repository
            let status = Command::new("git").arg("checkout").arg("main").status()?;
            if !status.success() {
                bail!("The 'git checkout main' command failed");
            }
        }
    }
    Ok(())
}
