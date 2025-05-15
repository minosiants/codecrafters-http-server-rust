use crate::{Context, Result};
use bytes::Bytes;
use std::fs::{create_dir_all, File as F};
use std::io::{Read, Write};
use std::path::Path;

pub trait FileOps {
    fn read(&self) -> Result<Bytes>;
    fn write(&self, data: Bytes) -> Result<()>;
    fn create_and_write(&self, data: Bytes) -> Result<()>;
}

impl FileOps for &str {
    fn read(&self) -> Result<Bytes> {
        read(self)
    }

    fn write(&self, data: Bytes) -> Result<()> {
        write(self, data)
    }
    fn create_and_write(&self, data: Bytes) -> Result<()> {
        create_and_write(&self, data)
    }
}
pub fn read(path: &str) -> Result<Bytes> {
    let mut file = F::open(path).context("Open file {path}")?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).context("Reading file")?;
    Ok(Bytes::from(buffer))
}

pub fn write(path: &str, data: Bytes) -> Result<()> {
    let mut file = F::create(path).with_context(|| "")?;
    file.write_all(&data).context("write to file")
}

pub fn create_and_write(path: &str, data: Bytes) -> Result<()> {
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
