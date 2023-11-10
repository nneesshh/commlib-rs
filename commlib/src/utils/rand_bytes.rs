use ring::rand::{SecureRandom, SystemRandom};

///
#[inline(always)]
pub fn rand_bytes(buf: &mut [u8]) -> Result<(), String> {
    rng().fill(buf).map_err(|e| e.to_string())?;
    Ok(())
}

///
#[inline(always)]
pub fn rand_bytes2(size: usize) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = vec![0; size];
    match rand_bytes(buf.as_mut_slice()) {
        Ok(_) => Ok(buf),
        Err(err) => Err(err),
    }
}

#[inline(always)]
fn rng() -> &'static dyn SecureRandom {
    use std::ops::Deref;

    lazy_static::lazy_static! {
        static ref RANDOM: SystemRandom = SystemRandom::new();
    }

    RANDOM.deref()
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{self, BufWriter, Read, Write};
    use tempfile::NamedTempFile;

    use super::*;

    fn get_writer<'a>(path: Option<&'a str>) -> Box<dyn Write> {
        let output = path
            .map(|path| {
                Box::new(File::create(path).expect("Unable to create file")) as Box<dyn Write>
            })
            .unwrap_or_else(|| Box::new(io::stdout()) as Box<dyn Write>);
        Box::new(BufWriter::new(output))
    }

    fn write_bytes<W: Write>(size: usize, mut writer: W) -> Result<(), String> {
        let bytes = rand_bytes(size)?;
        writer.write_all(&bytes).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Smoke test to generate a 32 bytes random value to some temporary file
    #[test]
    fn smoke_test() {
        let mut file = NamedTempFile::new().unwrap();
        {
            let writer = get_writer(Some("output"));
            let size = 128;
            write_bytes(size, writer).unwrap();
        }

        let mut bytes = Vec::<u8>::new();
        let actual_size = file.read_to_end(&mut bytes).unwrap();
        assert_eq!(32, actual_size);
    }
}
