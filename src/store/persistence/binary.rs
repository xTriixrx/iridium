use super::error::{PersistenceError, PersistenceResult};
use super::pipeline::PersistencePipeline;
use crate::store::buffer_snapshot::BufferSnapshot;
use std::convert::TryInto;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Cursor, Read, Write};
use std::path::Path;

const MAGIC: &[u8; 8] = b"IRDBUF\0\0";
const FORMAT_VERSION: u32 = 1;
#[cfg_attr(not(test), allow(dead_code))]
const HEADER_SIZE: usize = 32;

pub struct BinaryBufferDb;

impl BinaryBufferDb {
    pub fn load(
        path: &Path,
        pipeline: &PersistencePipeline,
    ) -> PersistenceResult<Vec<BufferSnapshot>> {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let header = FileHeader::read(&mut reader)?;
        if header.magic != *MAGIC {
            return Err(PersistenceError::InvalidMagic);
        }
        if header.version != FORMAT_VERSION {
            return Err(PersistenceError::UnsupportedVersion(header.version));
        }

        let mut payload = Vec::new();
        reader.read_to_end(&mut payload)?;
        if header.flags != pipeline.flags() {
            return Err(PersistenceError::UnsupportedFlags(header.flags));
        }
        let decoded = pipeline.decode(payload)?;
        let mut cursor = Cursor::new(decoded);

        let buffer_count: usize = header
            .buffer_count
            .try_into()
            .map_err(|_| PersistenceError::ValueOverflow("buffer_count"))?;
        let mut snapshots = Vec::with_capacity(buffer_count);

        for _ in 0..buffer_count {
            snapshots.push(Self::read_buffer(&mut cursor)?);
        }

        Ok(snapshots)
    }

    pub fn store(
        path: &Path,
        pipeline: &PersistencePipeline,
        snapshots: &[BufferSnapshot],
    ) -> PersistenceResult<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let mut temp_path = path.to_path_buf();
        temp_path.set_extension("tmp");

        let file = File::create(&temp_path)?;
        let mut writer = BufWriter::new(file);
        let payload = Self::encode_snapshots(snapshots)?;
        let transformed = pipeline.encode(payload)?;
        let header = FileHeader::new(pipeline.flags(), snapshots.len() as u64);
        header.write(&mut writer)?;
        writer.write_all(&transformed)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        drop(writer);

        fs::rename(&temp_path, path)?;

        Ok(())
    }

    fn encode_snapshots(snapshots: &[BufferSnapshot]) -> PersistenceResult<Vec<u8>> {
        let mut payload = Vec::new();
        for snapshot in snapshots {
            Self::write_buffer(&mut payload, snapshot)?;
        }
        Ok(payload)
    }

    fn read_buffer(reader: &mut dyn Read) -> PersistenceResult<BufferSnapshot> {
        let name_len = read_u32(reader)? as usize;
        let line_count = read_u32(reader)?;
        let mut flags = [0u8; 4];
        reader.read_exact(&mut flags)?;
        let _padding1 = read_u32(reader)?;

        let mut name_bytes = vec![0u8; name_len];
        reader.read_exact(&mut name_bytes)?;
        let name = String::from_utf8(name_bytes)?;

        let mut lines = Vec::with_capacity(line_count as usize);
        for _ in 0..line_count {
            lines.push(Self::read_line(reader)?);
        }

        Ok(BufferSnapshot::new(
            name,
            lines,
            flags[0] != 0,
            flags[1] != 0,
            flags[2] != 0,
        ))
    }

    fn write_buffer(writer: &mut dyn Write, snapshot: &BufferSnapshot) -> PersistenceResult<()> {
        let name_bytes = snapshot.name.as_bytes();
        let name_len: u32 = name_bytes
            .len()
            .try_into()
            .map_err(|_| PersistenceError::ValueOverflow("buffer name length"))?;
        let line_count: u32 = snapshot
            .lines
            .len()
            .try_into()
            .map_err(|_| PersistenceError::ValueOverflow("line count"))?;

        write_u32(writer, name_len)?;
        write_u32(writer, line_count)?;

        let flags = [
            bool_to_u8(snapshot.requires_name),
            bool_to_u8(snapshot.is_open),
            bool_to_u8(snapshot.dirty),
            0u8,
        ];
        writer.write_all(&flags)?;
        write_u32(writer, 0)?;

        writer.write_all(name_bytes)?;

        for line in &snapshot.lines {
            Self::write_line(writer, line)?;
        }

        Ok(())
    }

    fn read_line(reader: &mut dyn Read) -> PersistenceResult<String> {
        let line_len = read_u32(reader)? as usize;
        let _reserved = read_u32(reader)?;

        let mut bytes = vec![0u8; line_len];
        reader.read_exact(&mut bytes)?;
        let padding = padding_len(line_len);
        if padding > 0 {
            let mut sink = [0u8; 8];
            reader.read_exact(&mut sink[..padding])?;
        }

        Ok(String::from_utf8(bytes)?)
    }

    fn write_line(writer: &mut dyn Write, line: &str) -> PersistenceResult<()> {
        let bytes = line.as_bytes();
        let line_len: u32 = bytes
            .len()
            .try_into()
            .map_err(|_| PersistenceError::ValueOverflow("line length"))?;
        write_u32(writer, line_len)?;
        write_u32(writer, 0)?;
        writer.write_all(bytes)?;
        let padding = padding_len(bytes.len());
        if padding > 0 {
            writer.write_all(&ZERO_PADDING[..padding])?;
        }
        Ok(())
    }
}

struct FileHeader {
    magic: [u8; 8],
    version: u32,
    flags: u32,
    buffer_count: u64,
    reserved0: u64,
}

impl FileHeader {
    fn new(flags: u32, buffer_count: u64) -> Self {
        Self {
            magic: *MAGIC,
            version: FORMAT_VERSION,
            flags,
            buffer_count,
            reserved0: 0,
        }
    }

    fn read(reader: &mut dyn Read) -> PersistenceResult<Self> {
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        let version = read_u32(reader)?;
        let flags = read_u32(reader)?;
        let reserved0 = read_u64(reader)?;
        let buffer_count = read_u64(reader)?;
        Ok(Self {
            magic,
            version,
            flags,
            buffer_count,
            reserved0,
        })
    }

    fn write(&self, writer: &mut dyn Write) -> PersistenceResult<()> {
        writer.write_all(&self.magic)?;
        write_u32(writer, self.version)?;
        write_u32(writer, self.flags)?;
        write_u64(writer, self.reserved0)?;
        write_u64(writer, self.buffer_count)?;
        Ok(())
    }
}

fn write_u32(writer: &mut dyn Write, value: u32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn write_u64(writer: &mut dyn Write, value: u64) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn read_u32(reader: &mut dyn Read) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64(reader: &mut dyn Read) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn bool_to_u8(value: bool) -> u8 {
    if value { 1 } else { 0 }
}

fn padding_len(len: usize) -> usize {
    (8 - (len % 8)) % 8
}

const ZERO_PADDING: [u8; 8] = [0u8; 8];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_header() {
        let header = FileHeader::new(0xAB, 42);
        let mut buf = Vec::new();
        header.write(&mut buf).unwrap();
        assert_eq!(buf.len(), HEADER_SIZE);

        let mut cursor = Cursor::new(buf);
        let parsed = FileHeader::read(&mut cursor).unwrap();
        assert_eq!(parsed.magic, *MAGIC);
        assert_eq!(parsed.flags, 0xAB);
        assert_eq!(parsed.buffer_count, 42);
    }
}
