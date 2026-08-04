use std::io;

use zstd::{decode_all, encode_all};

/// Compresses a byte slice using Zstd at level 3 (default speed/ratio balance).
/// Returns `(compressed_bytes, "zstd")` if compression yielded size savings,
/// or `(raw_bytes, "none")` if compression did not reduce size.
pub fn compress_blob(raw_bytes: &[u8]) -> (Vec<u8>, String) {
    if raw_bytes.is_empty() {
        return (Vec::new(), "none".to_string());
    }

    match encode_all(raw_bytes, 3) {
        Ok(compressed) if compressed.len() < raw_bytes.len() => (compressed, "zstd".to_string()),
        _ => (raw_bytes.to_vec(), "none".to_string()),
    }
}

/// Decompresses a byte slice based on the specified compression algorithm.
pub fn decompress_blob(bytes: &[u8], compression: &str) -> io::Result<Vec<u8>> {
    match compression {
        "zstd" => decode_all(bytes),
        "none" | "" => Ok(bytes.to_vec()),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unsupported compression algorithm: {other}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compress_and_decompress_compressible_data() {
        let input = "hello world ".repeat(100).into_bytes();

        let (compressed, algo) = compress_blob(&input);
        assert_eq!(algo, "zstd");
        assert!(compressed.len() < input.len());

        let decompressed = decompress_blob(&compressed, &algo).unwrap();
        assert_eq!(input, decompressed);
    }

    #[tokio::test]
    async fn test_uncompressible_fallback() {
        let input = vec![1, 2, 3, 4, 5];
        let (output, algo) = compress_blob(&input);

        assert_eq!(algo, "none");
        assert_eq!(output, input);

        let decompressed = decompress_blob(&output, &algo).unwrap();
        assert_eq!(input, decompressed);
    }
}
