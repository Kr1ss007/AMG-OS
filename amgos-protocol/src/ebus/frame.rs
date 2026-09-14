//! AMGOS EventBus (e-bus) Binary Framing Protocol
//!
//! Provides strict framing with magic bytes, protocol versioning,
//! sequential message numbering, payload sizing, and checksum verification.

use serde::{de::DeserializeOwned, Serialize};
use std::io::{self, Read, Write};

pub const EBUS_MAGIC: [u8; 4] = [0x41, 0x4D, 0x47, 0x31]; // "AMG1"
pub const EBUS_PROTOCOL_VERSION: u16 = 1;
pub const HEADER_SIZE: usize = 16;
pub const MAX_PAYLOAD_SIZE: usize = 16 * 1024 * 1024; // 16 MB max frame

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub message_id: u32,
    pub payload_len: u32,
    pub checksum: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    InvalidMagic,
    UnsupportedVersion(u16),
    PayloadTooLarge(usize),
    ChecksumMismatch { expected: u16, found: u16 },
    Serialization(String),
    Deserialization(String),
    Io(String),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "Invalid e-bus frame magic bytes"),
            Self::UnsupportedVersion(v) => write!(f, "Unsupported e-bus version: {v}"),
            Self::PayloadTooLarge(s) => {
                write!(f, "Payload size {s} exceeds limit {MAX_PAYLOAD_SIZE}")
            }
            Self::ChecksumMismatch { expected, found } => {
                write!(
                    f,
                    "Checksum mismatch: expected 0x{expected:04X}, found 0x{found:04X}"
                )
            }
            Self::Serialization(e) => write!(f, "Serialization error: {e}"),
            Self::Deserialization(e) => write!(f, "Deserialization error: {e}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for FrameError {}

impl From<io::Error> for FrameError {
    fn from(err: io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

/// Compute a deterministic 16-bit Fletcher-style checksum for the payload
pub fn compute_checksum(data: &[u8]) -> u16 {
    let mut sum1: u32 = 0;
    let mut sum2: u32 = 0;
    for &byte in data {
        sum1 = (sum1 + byte as u32) % 255;
        sum2 = (sum2 + sum1) % 255;
    }
    ((sum2 << 8) | sum1) as u16
}

/// Encodes a typed payload into a complete framed packet
pub fn encode_frame<T: Serialize>(message_id: u32, payload: &T) -> Result<Vec<u8>, FrameError> {
    let serialized =
        bincode::serialize(payload).map_err(|e| FrameError::Serialization(e.to_string()))?;
    if serialized.len() > MAX_PAYLOAD_SIZE {
        return Err(FrameError::PayloadTooLarge(serialized.len()));
    }

    let checksum = compute_checksum(&serialized);
    let mut buffer = Vec::with_capacity(HEADER_SIZE + serialized.len());

    buffer.extend_from_slice(&EBUS_MAGIC);
    buffer.extend_from_slice(&EBUS_PROTOCOL_VERSION.to_be_bytes());
    buffer.extend_from_slice(&message_id.to_be_bytes());
    buffer.extend_from_slice(&(serialized.len() as u32).to_be_bytes());
    buffer.extend_from_slice(&checksum.to_be_bytes());
    buffer.extend_from_slice(&serialized);

    Ok(buffer)
}

/// Decodes a frame header from a reader
pub fn read_frame_header<R: Read>(reader: &mut R) -> Result<FrameHeader, FrameError> {
    let mut header_buf = [0u8; HEADER_SIZE];
    reader.read_exact(&mut header_buf)?;

    let mut magic = [0u8; 4];
    magic.copy_from_slice(&header_buf[0..4]);
    if magic != EBUS_MAGIC {
        return Err(FrameError::InvalidMagic);
    }

    let version = u16::from_be_bytes([header_buf[4], header_buf[5]]);
    if version != EBUS_PROTOCOL_VERSION {
        return Err(FrameError::UnsupportedVersion(version));
    }

    let message_id =
        u32::from_be_bytes([header_buf[6], header_buf[7], header_buf[8], header_buf[9]]);

    let payload_len = u32::from_be_bytes([
        header_buf[10],
        header_buf[11],
        header_buf[12],
        header_buf[13],
    ]);

    if (payload_len as usize) > MAX_PAYLOAD_SIZE {
        return Err(FrameError::PayloadTooLarge(payload_len as usize));
    }

    let checksum = u16::from_be_bytes([header_buf[14], header_buf[15]]);

    Ok(FrameHeader {
        magic,
        version,
        message_id,
        payload_len,
        checksum,
    })
}

/// Reads and decodes a typed payload from a reader according to the header
pub fn read_frame_payload<R: Read, T: DeserializeOwned>(
    reader: &mut R,
    header: &FrameHeader,
) -> Result<T, FrameError> {
    let mut payload = vec![0u8; header.payload_len as usize];
    reader.read_exact(&mut payload)?;

    let calculated = compute_checksum(&payload);
    if calculated != header.checksum {
        return Err(FrameError::ChecksumMismatch {
            expected: header.checksum,
            found: calculated,
        });
    }

    bincode::deserialize(&payload).map_err(|e| FrameError::Deserialization(e.to_string()))
}

/// Writes a framed message directly to a writer
pub fn write_frame<W: Write, T: Serialize>(
    writer: &mut W,
    message_id: u32,
    payload: &T,
) -> Result<(), FrameError> {
    let frame = encode_frame(message_id, payload)?;
    writer.write_all(&frame)?;
    writer.flush()?;
    Ok(())
}
