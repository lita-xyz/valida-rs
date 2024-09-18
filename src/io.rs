use serde::{de::DeserializeOwned, Serialize};
use std::error::Error;

extern "C" {
    pub fn getchar() -> u32;
    pub fn putchar(c: u32) -> u32;
}

/// Read from the input tape until we hit a specific character.
pub fn read_until(c: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut result = Vec::new();
    loop {
        let input = unsafe { getchar() as u8 };
        if input == c {
            // All done, found the character to stop at.
            break;
        }
        result.push(input);
    }
    Ok(result)
}

/// Read n bytes from the input tape.
pub fn read_n(n: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok((0..n).map(|_| unsafe { getchar() as u8 }).collect())
}

/// Write the contents of a vector to the output tape.
pub fn write_vec(v: &Vec<u8>) -> Result<(), Box<dyn Error>> {
    v.iter().for_each(|c| { unsafe { putchar(*c as u32); } });
    Ok(())
}

/// Construct a deserializable object from bytes read off the input tape.
pub fn read<T: DeserializeOwned>() -> Result<T, Box<dyn Error>> {
    // First line should be an integer specifying how many characters the serialized object takes
    // up on the input tape.
    let n: usize = std::str::from_utf8(&read_until('\n' as u8)?)?.parse()?;
    // Now read the actual bytes relating to the serialized object.
    let bytes = read_n(n)?;
    // Deserialize the object.
    bincode::deserialize(&bytes).map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Serialize an object and write it to the output tape.
pub fn write<T: Serialize>(value: &T) -> Result<(), Box<dyn Error>> {
    // Serialize the object to discover how many bytes it will take.
    let bytes = bincode::serialize(value)?;
    // Write an integer specifying the number of bytes used for the serialized object, plus a
    // newline.
    let mut n = bytes.len().to_string().into_bytes();
    n.push('\n' as u8);
    write_vec(&n)?;
    // Write the serialized object to the output tape.
    write_vec(&bytes)?;
    Ok(())
}
