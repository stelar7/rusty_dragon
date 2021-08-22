use nom::{
    bytes::complete::tag,
    error::VerboseError,
    number::complete::{le_u32, le_u64, le_u8},
    sequence::tuple,
};

#[path = "../macros.rs"]
mod macros;

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
    bundle_offset: u32,
    language_offset: u32,
    file_offset: u32,
    folder_offset: u32,
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
struct Language {
    id: u32,
    name: String,
}

#[derive(Debug, PartialEq, Eq)]
struct FileEntry {
    id: u64,
    name: String,
    symlink: String,
    directory_id: u64,
    size: u32,
    language: u32,
    chunk_ids: Vec<u64>,
}

#[derive(Debug, PartialEq, Eq)]
struct Directory {
    id: u64,
    parent_id: u64,
    name: String,
}

#[derive(Debug, PartialEq, Eq)]
struct Body {
    bundles: Vec<Bundle>,
    languages: Vec<Language>,
    files: Vec<FileEntry>,
    directories: Vec<Directory>,
}

pub fn parse(input: &[u8]) -> File {
    let header = header(input);

    let body_data = &input[header.offset as usize..(header.offset + header.length) as usize];

    let decompressed = zstd::decode_all(body_data).unwrap();

    let offsets = offset_map(&decompressed);
    let bundles = bundles(&decompressed[offsets.bundle_offset as usize..]);
    let languages = languages(&decompressed[offsets.language_offset as usize..]);
    let files = files(&decompressed[offsets.file_offset as usize..]);
    println!("{:?}", files);

    let directories = directories(&decompressed[offsets.folder_offset as usize..]);

    let body = Body { bundles, languages, files, directories };

    File { header, body }
}

fn header(input: &[u8]) -> Header {
    let (magic, major, minor, unknown, signature_type, offset, length, manifest_id, decompressed_length) =
        crate::parse_tuple!((tag("RMAN"), le_u8, le_u8, le_u8, le_u8, le_u32, le_u32, le_u64, le_u32,), input);

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
    }
}

fn offset_map(input: &[u8]) -> OffsetMap {
    let header_offset = crate::parse_single!(le_u32, input);

    let (_, bundle_offset, language_offset, file_offset, folder_offset) =
        crate::parse_tuple!((le_u32, le_u32, le_u32, le_u32, le_u32,), &input[header_offset as usize..]);

    OffsetMap {
        bundle_offset: header_offset + bundle_offset + 4,
        language_offset: header_offset + language_offset + 8,
        file_offset: header_offset + file_offset + 12,
        folder_offset: header_offset + folder_offset + 16,
    }
}

fn bundles(input: &[u8]) -> Vec<Bundle> {
    let mut bundles = Vec::<Bundle>::new();
    let bundle_count = crate::parse_single!(le_u32, input);

    for i in 0..bundle_count {
        let input_position = 4 + 4 * i;
        let bundle_offset = crate::parse_single!(le_u32, &input[input_position as usize..]);

        let bundle_data = &input[(input_position + bundle_offset) as usize..];
        let (_, header_size, bundle_id) = crate::parse_tuple!((le_u32, le_u32, le_u64), bundle_data);

        let chunk_count_offset = 4 + header_size;
        let chunk_count = crate::parse_single!(le_u32, &bundle_data[chunk_count_offset as usize..]);
        let mut chunks = Vec::<Chunk>::new();

        for j in 0..chunk_count {
            let chunk_position = 4 + 4 * j;
            let chunk_data_position = chunk_count_offset + chunk_position;
            let chunk_offset = crate::parse_single!(le_u32, &bundle_data[chunk_data_position as usize..]);

            let chunk_data = &bundle_data[(chunk_data_position + chunk_offset) as usize..];
            let (_, compressed_size, uncompressed_size, chunk_id) =
                crate::parse_tuple!((le_u32, le_u32, le_u32, le_u64), chunk_data);

            chunks.push(Chunk { compressed_size, uncompressed_size, chunk_id });
        }

        bundles.push(Bundle { bundle_id, chunks });
    }

    bundles
}

fn languages(input: &[u8]) -> Vec<Language> {
    let mut languages = Vec::<Language>::new();
    let language_count = crate::parse_single!(le_u32, input);

    for i in 0..language_count {
        let language_position = 4 + 4 * i;
        let language_offset = crate::parse_single!(le_u32, &input[language_position as usize..]);

        let language_data = &input[(language_position + language_offset) as usize..];
        let (_, language_id, language_name_offset) = crate::parse_tuple!((le_u32, le_u32, le_u32), language_data);

        let language_name_data = &language_data[8 + language_name_offset as usize..];
        let language_name_size = crate::parse_single!(le_u32, language_name_data);

        let language_name =
            String::from_utf8_lossy(&language_name_data[4..4 + language_name_size as usize]).into_owned();

        languages.push(Language { id: language_id, name: language_name });
    }

    languages
}

fn files(input: &[u8]) -> Vec<FileEntry> {
    let mut files = Vec::<FileEntry>::new();

    //todo!("File parsing");

    files
}

fn directories(input: &[u8]) -> Vec<Directory> {
    let mut directories = Vec::<Directory>::new();

    //todo!("Directory parsing");

    directories
}
