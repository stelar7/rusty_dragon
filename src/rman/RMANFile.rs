use nom::{
    bytes::complete::tag,
    error::{context, VerboseError},
    number::complete::{le_u32, le_u64, le_u8},
    sequence::tuple,
    AsBytes, IResult,
};

#[path = "../macros.rs"]
mod macros;

type Res<T, U> = IResult<T, U, VerboseError<T>>;

#[derive(Debug, PartialEq, Eq)]
pub struct File {
    header: Header,
    body: Body,
}

#[derive(Debug, PartialEq, Eq)]
struct Header {
    magic: String,
    major: u8,
    minor: u8,
    unknown: u8,
    signature_type: u8,
    offset: u32,
    length: u32,
    manifest_id: u64,
    decompressed_length: u32,
}

#[derive(Debug, PartialEq, Eq)]
struct OffsetMap {
    table_offset: u32,
    bundle_offset: u32,
    language_offset: u32,
    file_offset: u32,
    folder_offset: u32,
    key_offset: u32,
    unknown_offset: u32,
}

#[derive(Debug, PartialEq, Eq)]
struct Bundle {
    bundle_id: u64,
    chunks: Vec<Chunk>,
}

#[derive(Debug, PartialEq, Eq)]
struct Chunk {
    compressed_size: u32,
    uncompressed_size: u32,
    chunk_id: u64,
}

#[derive(Debug, PartialEq, Eq)]
struct Body {}

pub fn parse(input: &[u8]) -> File {
    let header = header(input).unwrap().1;
    let body_data = &input[header.offset as usize..(header.offset + header.length) as usize];
    let decompressed = zstd::decode_all(body_data).unwrap();
    let offsets = offset_map(decompressed.as_bytes()).unwrap().1;
    let bundles = bundles(&decompressed[offsets.bundle_offset as usize..]);

    println!("{:?}", bundles);

    let body = Body {};
    File { header, body }
}

fn header(input: &[u8]) -> Res<&[u8], Header> {
    context(
        "RMANHeader",
        tuple((
            tag("RMAN"),
            le_u8,
            le_u8,
            le_u8,
            le_u8,
            le_u32,
            le_u32,
            le_u64,
            le_u32,
        )),
    )(input)
    .map(|(next, res)| {
        let (
            magic,
            major,
            minor,
            unknown,
            signature_type,
            offset,
            length,
            manifest_id,
            decompressed_length,
        ) = res;
        (
            next,
            Header {
                magic: String::from_utf8_lossy(magic).into_owned(),
                major,
                minor,
                unknown,
                signature_type,
                offset,
                length,
                manifest_id,
                decompressed_length,
            },
        )
    })
}

fn offset_map(input: &[u8]) -> Res<&[u8], OffsetMap> {
    let header_offset = crate::parse_single!(le_u32, input);

    context(
        "RMANBody->headers",
        tuple((le_u32, le_u32, le_u32, le_u32, le_u32, le_u32, le_u32)),
    )(&input[header_offset as usize..])
    .map(|(next, res)| {
        let (
            table_offset,
            bundle_offset,
            language_offset,
            file_offset,
            folder_offset,
            key_offset,
            unknown_offset,
        ) = res;
        (
            next,
            OffsetMap {
                table_offset,
                bundle_offset: header_offset + bundle_offset + 4,
                language_offset: header_offset + language_offset + 8,
                file_offset: header_offset + file_offset + 12,
                folder_offset: header_offset + folder_offset + 16,
                key_offset: header_offset + key_offset + 20,
                unknown_offset: header_offset + unknown_offset + 24,
            },
        )
    })
}

fn bundles(input: &[u8]) -> Res<&[u8], Vec<Bundle>> {
    let mut bundles = Vec::<Bundle>::new();
    let bundle_count = crate::parse_single!(le_u32, input);

    for i in 0..bundle_count {
        let input_position = 4 + 4 * i;
        let bundle_offset = crate::parse_single!(le_u32, &input[input_position as usize..]);

        let bundle_data = &input[(input_position + bundle_offset) as usize..];
        let header_size = crate::parse_single!(le_u32, &bundle_data[4..]);
        let bundle_id = crate::parse_single!(le_u64, &bundle_data[8..]);

        let chunk_count = crate::parse_single!(le_u32, &bundle_data[(4 + header_size) as usize..]);
        let mut chunks = Vec::<Chunk>::new();

        for j in 0..chunk_count {
            let chunk_position = 4 + 4 * j;
            let chunk_offset = crate::parse_single!(
                le_u32,
                &bundle_data[(4 + header_size + chunk_position) as usize..]
            );

            let chunk_data =
                &bundle_data[(4 + header_size + chunk_position + chunk_offset) as usize..];
            let compressed_size = crate::parse_single!(le_u32, &chunk_data[4..]);
            let uncompressed_size = crate::parse_single!(le_u32, &chunk_data[8..]);
            let chunk_id = crate::parse_single!(le_u64, &chunk_data[12..]);

            chunks.push(Chunk {
                compressed_size,
                uncompressed_size,
                chunk_id,
            })
        }

        bundles.push(Bundle { bundle_id, chunks })
    }

    Ok((input, bundles))
}
