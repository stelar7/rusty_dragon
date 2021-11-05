use nom::{
    bytes::complete::tag,
    error::VerboseError,
    number::complete::{le_u16, le_u32, le_u64, le_u8},
    sequence::tuple,
};

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct File {
    header: Header,
    content: Vec<Content>,
}

#[derive(Debug, PartialEq, Eq, Serialize, EnumDiscriminants)]
enum HeaderVersion {
    V1 { entry_offset: u16, entry_size: u16 },
    V2 { ecdsa: Vec<u8>, file_checksum: u64, entry_offset: u16, entry_size: u16 },
    V3 { ecdsa: Vec<u8>, file_checksum: u64 },
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Header {
    major: u8,
    minor: u8,
    file_count: u32,
    version: HeaderVersion,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Content {
    hash: u64,
    data_offset: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    compression_type: CompressionType,
    version: ContentVersion,
}

#[derive(Debug, PartialEq, Eq, Serialize, EnumDiscriminants)]
enum ContentVersion {
    V1 {},
    V2 { is_duplicate: bool, sha256: u64 },
}

#[derive(Debug, PartialEq, Eq, Serialize, FromPrimitive)]
enum CompressionType {
    NONE = 0,
    GZIP = 1,
    REFERENCE = 2,
    ZSTD = 3,
}

pub fn parse(input: &[u8]) -> File {
    let header = header(input);
    let content = content(input, header.major, header.file_count);

    File { header, content }
}

fn header(input: &[u8]) -> Header {
    let (_, major, minor) = crate::parse_tuple!((tag("RW"), le_u8, le_u8), input);

    if major == 1 {
        let (entry_offset, entry_size, file_count) = crate::parse_tuple!((le_u16, le_u16, le_u32), &input[4..]);
        return Header { major, minor, file_count, version: HeaderVersion::V1 { entry_offset, entry_size } };
    }

    if major == 2 {
        let ecdsa_length = crate::parse_single!(le_u8, input);
        let ecdsa = input[5..5 + ecdsa_length as usize].to_vec();
        let ecdsa_end = 5 + 83;

        let (file_checksum, entry_offset, entry_size, file_count) =
            crate::parse_tuple!((le_u64, le_u16, le_u16, le_u32), &input[ecdsa_end as usize..]);

        return Header { major, minor, file_count, version: HeaderVersion::V2 { ecdsa, file_checksum, entry_offset, entry_size } };
    }

    if major == 3 {
        let ecdsa = input[4..4 + 256_usize].to_vec();
        let (file_checksum, file_count) = crate::parse_tuple!((le_u64, le_u32), &input[4 + 256_usize..]);
        return Header { major, minor, file_count, version: HeaderVersion::V3 { ecdsa, file_checksum } };
    }

    panic!("Invalid major version for wad file");
}

fn content(input: &[u8], major: u8, file_count: u32) -> Vec<Content> {
    let (data_start, entry_size) = match major {
        1 => (4 + 2 + 2 + 4, 24),
        2 => (4 + 1 + 83 + 8 + 2 + 2 + 4, 32),
        3 => (4 + 256 + 8 + 4, 32),
        _ => {
            unreachable!();
        }
    };

    let mut entries = Vec::<Content>::new();

    for offset_multiplier in 0..file_count {
        let entry_offset = data_start + (offset_multiplier * entry_size);
        let (hash, data_offset, compressed_size, uncompressed_size) =
            crate::parse_tuple!((le_u64, le_u32, le_u32, le_u32), &input[entry_offset as usize..]);
        let compression_value: u8 = if major == 1 {
            crate::parse_single!(le_u32, &input[(entry_offset + 20) as usize..]) as u8
        } else {
            crate::parse_single!(le_u8, &input[(entry_offset + 20) as usize..])
        };
        let compression_type = CompressionType::from_u8(compression_value).unwrap();

        if major == 1 {
            entries.push(Content {
                hash,
                data_offset,
                compressed_size,
                uncompressed_size,
                compression_type,
                version: ContentVersion::V1 {},
            });
            continue;
        }

        let (duplicate, _, sha256) = crate::parse_tuple!((le_u8, le_u16, le_u64), &input[(entry_offset + 21) as usize..]);
        entries.push(Content {
            hash,
            data_offset,
            compressed_size,
            uncompressed_size,
            compression_type,
            version: ContentVersion::V2 { is_duplicate: duplicate > 0, sha256 },
        });
    }

    entries
}
