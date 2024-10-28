use bincode::Options;
use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error, io::Read};

extern "C" {
    pub fn getchar() -> u32;
    pub fn putchar(c: u32) -> u32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InputTape;

impl Read for InputTape {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        (0..buf.len()).for_each(|i| {
            buf[i] = unsafe { getchar() as u8 };
        });
        Ok(buf.len())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutputTape;

impl OutputTape {
    pub fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        (0..buf.len()).for_each(|i| unsafe {
            putchar(buf[i] as u32);
        });
        Ok(buf.len())
    }
}

/// Mimic std::io::println
pub fn println(s: &str) {
    let length = s.len();
    let bytes = s.as_bytes();
    (0..length).for_each(|i| unsafe {
        putchar(bytes[i] as u32);
    });
    unsafe { putchar('\n' as u32) };
}
/// Reads a single line of input from stdin and returns it as a generic type T.
pub fn read_line<T>() -> Result<T, Box<dyn Error>>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::error::Error + 'static,
{
    let input = read_until(b'\n')?;
    let trimmed = std::str::from_utf8(&input)?.trim();
    match trimmed.parse() {
        Ok(value) => Ok(value),
        Err(e) => Err(Box::new(e)),
    }
}

/// Mimic std::fs::read https://doc.rust-lang.org/std/fs/fn.read.html
/// Read from the input tape until EOF and return the contents as a Vec<u8>.
pub fn read() -> Result<Vec<u8>, Box<dyn Error>> {
    let mut result = Vec::new();
    loop {
        let input = unsafe { getchar() as u32 };
        if input == u32::MAX {
            // EOF reached
            break;
        }
        result.push(input as u8);
    }
    Ok(result)
}

/// Read from the input tape until we hit EOF or a specific character.
pub fn read_until(stop_char: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut result = Vec::new();
    loop {
        let input = unsafe { getchar() as u32 };
        if input == u32::MAX {
            // EOF reached
            break;
        }
        let input_byte = input as u8;
        if input_byte == stop_char {
            // Found the character to stop at
            break;
        }
        result.push(input_byte);
    }
    Ok(result)
}

/// Read n bytes from the input tape.
pub fn read_n(n: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok((0..n).map(|_| unsafe { getchar() as u8 }).collect())
}

/// Write the contents of a vector to the output tape.
pub fn write_vec(v: impl AsRef<[u8]>) -> Result<(), Box<dyn Error>> {
    v.as_ref().iter().for_each(|c| unsafe {
        putchar(*c as u32);
    });
    Ok(())
}

/// Construct a deserializable object from bytes read off the input tape.
pub fn read_and_deserialize<T: DeserializeOwned>() -> Result<T, Box<dyn Error>> {
    let bytes = match read() {
        Ok(b) => b,
        Err(e) => {
            println(&format!("Error reading bytes: {}", e));
            return Err(e);
        }
    };

    // Deserialize the object.
    bincode::options()
        .with_fixint_encoding()
        .with_little_endian()
        .deserialize(&bytes)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Serialize an object and write it to the output tape.
pub fn write<T: Serialize>(value: &T) -> Result<(), Box<dyn Error>> {
    // Serialize the object to discover how many bytes it will take.
    let bytes = bincode::options()
        .with_fixint_encoding()
        .with_little_endian()
        .serialize(value)?;
    // Write an integer specifying the number of bytes used for the serialized object, plus a
    // newline.
    let mut n = bytes.len().to_string().into_bytes();
    n.push(b'\n');
    write_vec(&n)?;
    // Write the serialized object to the output tape.
    write_vec(&bytes)?;
    Ok(())
}
