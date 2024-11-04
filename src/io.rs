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