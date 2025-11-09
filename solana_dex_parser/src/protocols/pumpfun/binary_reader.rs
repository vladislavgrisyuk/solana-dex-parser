use std::io::Cursor;

use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt};

pub struct BinaryReader {
    buffer: Vec<u8>,
    offset: usize,
}

impl BinaryReader {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            buffer: data,
            offset: 0,
        }
    }

    pub fn read_fixed_array(&mut self, length: usize) -> Result<Vec<u8>> {
        self.check_bounds(length)?;
        let slice = self.buffer[self.offset..self.offset + length].to_vec();
        self.offset += length;
        Ok(slice)
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        self.check_bounds(1)?;
        let value = self.buffer[self.offset];
        self.offset += 1;
        Ok(value)
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        self.check_bounds(2)?;
        let mut cursor = Cursor::new(&self.buffer[self.offset..self.offset + 2]);
        let value = cursor.read_u16::<LittleEndian>()?;
        self.offset += 2;
        Ok(value)
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        self.check_bounds(8)?;
        let mut cursor = Cursor::new(&self.buffer[self.offset..self.offset + 8]);
        let value = cursor.read_u64::<LittleEndian>()?;
        self.offset += 8;
        Ok(value)
    }

    pub fn read_i64(&mut self) -> Result<i64> {
        self.check_bounds(8)?;
        let mut cursor = Cursor::new(&self.buffer[self.offset..self.offset + 8]);
        let value = cursor.read_i64::<LittleEndian>()?;
        self.offset += 8;
        Ok(value)
    }

    pub fn read_string(&mut self) -> Result<String> {
        self.check_bounds(4)?;
        let mut cursor = Cursor::new(&self.buffer[self.offset..self.offset + 4]);
        let length = cursor.read_u32::<LittleEndian>()? as usize;
        self.offset += 4;
        self.check_bounds(length)?;
        let bytes = self.buffer[self.offset..self.offset + length].to_vec();
        self.offset += length;
        String::from_utf8(bytes).map_err(|err| anyhow!("failed to read string: {err}"))
    }

    pub fn read_pubkey(&mut self) -> Result<String> {
        let bytes = self.read_fixed_array(32)?;
        Ok(bs58::encode(bytes).into_string())
    }

    pub fn remaining(&self) -> usize {
        self.buffer.len().saturating_sub(self.offset)
    }

    fn check_bounds(&self, length: usize) -> Result<()> {
        if self.offset + length > self.buffer.len() {
            return Err(anyhow!(
                "buffer overflow: trying to read {} bytes at offset {} from buffer of length {}",
                length,
                self.offset,
                self.buffer.len()
            ));
        }
        Ok(())
    }
}
