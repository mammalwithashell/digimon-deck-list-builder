//! Minimal NumPy `.npy` (format version 1.0) writer + reader.
//!
//! The self-play shard boundary (design D9) is plain files consumed by the
//! Python trainer via `numpy.load`, so we hand-roll the trivial v1.0 format
//! instead of adding a crate dependency. Only the dtypes the shards need
//! are supported: little-endian `f4`, `i4`, `i8`, `u1`, in 1-D or 2-D
//! C-order. The readers exist for integration tests and Rust-side tooling;
//! the trainer reads with numpy directly.

use std::io::Write;
use std::path::Path;

const MAGIC: &[u8] = b"\x93NUMPY\x01\x00";

/// Build the v1.0 header for `descr` (e.g. `"<f4"`) and `shape`, padded so
/// the data section starts 64-byte aligned (the spec's recommendation).
fn header(descr: &str, shape: &[usize]) -> Vec<u8> {
    let shape_str = match shape {
        [n] => format!("({n},)"),
        dims => {
            let inner: Vec<String> = dims.iter().map(|d| d.to_string()).collect();
            format!("({})", inner.join(", "))
        }
    };
    let mut dict = format!("{{'descr': '{descr}', 'fortran_order': False, 'shape': {shape_str}, }}");
    // total = MAGIC(8) + len(2) + dict + '\n', padded to a multiple of 64.
    let unpadded = MAGIC.len() + 2 + dict.len() + 1;
    let pad = (64 - unpadded % 64) % 64;
    dict.push_str(&" ".repeat(pad));
    dict.push('\n');

    let mut out = Vec::with_capacity(MAGIC.len() + 2 + dict.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(dict.len() as u16).to_le_bytes());
    out.extend_from_slice(dict.as_bytes());
    out
}

fn write_all(path: &Path, head: &[u8], body: &[u8]) -> Result<(), String> {
    let mut f = std::fs::File::create(path)
        .map_err(|e| format!("creating {}: {e}", path.display()))?;
    f.write_all(head)
        .and_then(|()| f.write_all(body))
        .map_err(|e| format!("writing {}: {e}", path.display()))
}

pub fn write_f32_2d(path: &Path, rows: usize, cols: usize, data: &[f32]) -> Result<(), String> {
    assert_eq!(data.len(), rows * cols, "f32 2-D payload length mismatch");
    let body: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
    write_all(path, &header("<f4", &[rows, cols]), &body)
}

pub fn write_f32_1d(path: &Path, data: &[f32]) -> Result<(), String> {
    let body: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
    write_all(path, &header("<f4", &[data.len()]), &body)
}

pub fn write_i32_1d(path: &Path, data: &[i32]) -> Result<(), String> {
    let body: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
    write_all(path, &header("<i4", &[data.len()]), &body)
}

pub fn write_i64_1d(path: &Path, data: &[i64]) -> Result<(), String> {
    let body: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
    write_all(path, &header("<i8", &[data.len()]), &body)
}

pub fn write_u8_1d(path: &Path, data: &[u8]) -> Result<(), String> {
    write_all(path, &header("|u1", &[data.len()]), data)
}

/// Parsed `.npy` payload: shape + raw little-endian body bytes.
pub struct NpyArray {
    pub descr: String,
    pub shape: Vec<usize>,
    pub body: Vec<u8>,
}

/// Read any of the dtypes this module writes. Strict on the parts we
/// produce (v1.0, C-order); not a general-purpose npy reader.
pub fn read(path: &Path) -> Result<NpyArray, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    if bytes.len() < 10 || &bytes[..8] != MAGIC {
        return Err(format!("{}: not an npy v1.0 file", path.display()));
    }
    let hlen = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    let head = std::str::from_utf8(&bytes[10..10 + hlen])
        .map_err(|e| format!("{}: header not UTF-8: {e}", path.display()))?;
    let descr = extract_quoted(head, "'descr':")
        .ok_or_else(|| format!("{}: no descr in header", path.display()))?;
    let shape_src = head
        .split("'shape':")
        .nth(1)
        .and_then(|s| s.split('(').nth(1))
        .and_then(|s| s.split(')').next())
        .ok_or_else(|| format!("{}: no shape in header", path.display()))?;
    let shape: Vec<usize> = shape_src
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<usize>().map_err(|e| format!("bad dim {s}: {e}")))
        .collect::<Result<_, _>>()?;
    Ok(NpyArray {
        descr,
        shape,
        body: bytes[10 + hlen..].to_vec(),
    })
}

fn extract_quoted(head: &str, key: &str) -> Option<String> {
    let after = head.split(key).nth(1)?;
    let start = after.find('\'')? + 1;
    let end = start + after[start..].find('\'')?;
    Some(after[start..end].to_string())
}

impl NpyArray {
    pub fn as_f32(&self) -> Result<Vec<f32>, String> {
        if self.descr != "<f4" {
            return Err(format!("expected <f4, got {}", self.descr));
        }
        Ok(self
            .body
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect())
    }

    pub fn as_i32(&self) -> Result<Vec<i32>, String> {
        if self.descr != "<i4" {
            return Err(format!("expected <i4, got {}", self.descr));
        }
        Ok(self
            .body
            .chunks_exact(4)
            .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect())
    }

    pub fn as_i64(&self) -> Result<Vec<i64>, String> {
        if self.descr != "<i8" {
            return Err(format!("expected <i8, got {}", self.descr));
        }
        Ok(self
            .body
            .chunks_exact(8)
            .map(|c| i64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
            .collect())
    }

    pub fn as_u8(&self) -> Result<Vec<u8>, String> {
        if self.descr != "|u1" {
            return Err(format!("expected |u1, got {}", self.descr));
        }
        Ok(self.body.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_2d_roundtrip() {
        let dir = std::env::temp_dir().join("digimon_npy_test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("rt.npy");
        let data: Vec<f32> = (0..12).map(|i| i as f32 * 0.5).collect();
        write_f32_2d(&p, 3, 4, &data).unwrap();
        let arr = read(&p).unwrap();
        assert_eq!(arr.shape, vec![3, 4]);
        assert_eq!(arr.as_f32().unwrap(), data);
        // Data section must start 64-aligned per the header contract.
        let raw = std::fs::read(&p).unwrap();
        let hlen = u16::from_le_bytes([raw[8], raw[9]]) as usize;
        assert_eq!((10 + hlen) % 64, 0);
    }

    #[test]
    fn int_and_u8_roundtrip() {
        let dir = std::env::temp_dir().join("digimon_npy_test");
        std::fs::create_dir_all(&dir).unwrap();
        let p64 = dir.join("i64.npy");
        write_i64_1d(&p64, &[0, 7, 1 << 40]).unwrap();
        let a = read(&p64).unwrap();
        assert_eq!(a.shape, vec![3]);
        assert_eq!(a.as_i64().unwrap(), vec![0, 7, 1 << 40]);

        let p32 = dir.join("i32.npy");
        write_i32_1d(&p32, &[-1, 2192]).unwrap();
        assert_eq!(read(&p32).unwrap().as_i32().unwrap(), vec![-1, 2192]);

        let pu = dir.join("u8.npy");
        write_u8_1d(&pu, &[0, 1, 255]).unwrap();
        assert_eq!(read(&pu).unwrap().as_u8().unwrap(), vec![0, 1, 255]);
    }
}
