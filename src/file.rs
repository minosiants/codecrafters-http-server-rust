use crate::{Context, Result};
use std::fs::{create_dir_all, File as F};
use std::io::{Read, Write};
use std::path::Path;

pub trait FileOps {
    fn read(&self) -> Result<Vec<u8>>;
    fn write(&self, data: Vec<u8>) -> Result<()>;
    fn create_and_write(&self, data: Vec<u8>) -> Result<()>;
}

impl FileOps for &str {
    fn read(&self) -> Result<Vec<u8>> {
        read(self)
    }

    fn write(&self, data: Vec<u8>) -> Result<()> {
        write(self, data)
    }
    fn create_and_write(&self, data: Vec<u8>) -> Result<()> {
        create_and_write(&self, data)
    }
}
pub fn read(path: &str) -> Result<Vec<u8>> {
    let mut file = F::open(path).context("Open file {path}")?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).context("Reading file")?;
    Ok(buffer)
}

pub fn write(path: &str, data: Vec<u8>) -> Result<()> {
    let mut file = F::create(path).with_context(|| "")?;
    file.write_all(&data).context("write to file")
}

pub fn create_and_write(path: &str, data: Vec<u8>) -> Result<()> {
    create(path)?;
    write(path, data)
}
fn create(path: &str) -> Result<()> {
    let path = Path::new(path);
    if let Some(parent) = path.parent() {
        create_dir_all(parent).with_context(|| "")?
    };
    Ok(())
}
