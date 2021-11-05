use nom::{
    bytes::complete::tag,
    error::VerboseError,
    number::complete::{le_i32, le_u16, le_u32, le_u64, le_u8},
    sequence::tuple,
};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct File {
    header: Header,
    body: Body,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
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

#[derive(Debug, PartialEq, Eq, Serialize)]
struct OffsetMap {
    bundle_offset: u32,
    language_offset: u32,
    file_offset: u32,
    folder_offset: u32,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Bundle {
    bundle_id: u64,
    chunks: Vec<Chunk>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Chunk {
    compressed_size: u32,
    uncompressed_size: u32,
    chunk_id: u64,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Language {
    id: u8,
    name: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct FileEntry {
    id: u64,
    name: String,
    symlink: String,
    directory_id: u64,
    size: u32,
    language: u32,
    chunk_ids: Vec<u64>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Directory {
    id: u64,
    parent_id: u64,
    name: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
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
    let bundles = bundles(&decompressed, offsets.bundle_offset);
    let languages = languages(&decompressed, offsets.language_offset);
    let directories = directories(&decompressed, offsets.folder_offset);
    let files = files(&decompressed, offsets.file_offset);

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

fn parse_vector<F>(input: &[u8], table_offset: u32, parts: &[&str], f: &mut F)
where
    F: FnMut(u32, HashMap<String, u16>),
{
    let count = crate::parse_single!(le_u32, &input[table_offset as usize..]);

    for i in 0..count {
        let entry_position = 4 + 4 * i;
        let offset = crate::parse_single!(le_u32, &input[(table_offset + entry_position) as usize..]);

        let entry_data_offset = entry_position + offset + table_offset;

        let entry_offsets = parse_vtable(input, entry_data_offset, parts);
        f(entry_data_offset, entry_offsets);
    }
}

fn parse_long_vector<F>(input: &[u8], start_offset: u32, f: &mut F)
where
    F: FnMut(u64),
{
    let count = crate::parse_single!(le_u32, &input[start_offset as usize..]);

    for i in 0..count {
        let entry_position = 4 + 8 * i;
        let value = crate::parse_single!(le_u64, &input[(start_offset + entry_position) as usize..]);
        f(value);
    }
}

fn parse_vtable(input: &[u8], table_offset: u32, entries: &[&str]) -> HashMap<String, u16> {
    let offset = crate::parse_single!(le_i32, &input[table_offset as usize..]);
    let vtable_data = &input[(table_offset as i32 - offset) as usize..];

    let mut offsets = HashMap::<String, u16>::new();
    for (index, &element) in entries.iter().enumerate() {
        let value = crate::parse_single!(le_u16, &vtable_data[(index * 2) as usize..]);
        offsets.insert(element.to_owned(), value);
    }

    offsets
}

fn bundles(input: &[u8], bundles_start: u32) -> Vec<Bundle> {
    let mut bundles = Vec::<Bundle>::new();

    let mut parse_single_bundle = |start_offset: u32, entry_offsets: HashMap<String, u16>| {
        let bundle_id_offset = entry_offsets.get("bundle_id").unwrap().to_owned();
        let bundle_id = crate::parse_single!(le_u64, &input[(start_offset + bundle_id_offset as u32) as usize..]);

        let mut chunks_list = Vec::<Chunk>::new();
        let chunks_offset = entry_offsets.get("chunks").unwrap().to_owned();
        let mut parse_single_chunk = |start_offset: u32, entry_offsets: HashMap<String, u16>| {
            let chunk_id_offset = entry_offsets.get("chunk_id").unwrap().to_owned();
            let chunk_id = crate::parse_single!(le_u64, &input[(start_offset + chunk_id_offset as u32) as usize..]);

            let compressed_size_offset = entry_offsets.get("compressed_size").unwrap().to_owned();
            let compressed_size = crate::parse_single!(le_u32, &input[(start_offset + compressed_size_offset as u32) as usize..]);

            let uncompressed_size_offset = entry_offsets.get("uncompressed_size").unwrap().to_owned();
            let uncompressed_size = crate::parse_single!(le_u32, &input[(start_offset + uncompressed_size_offset as u32) as usize..]);

            chunks_list.push(Chunk { chunk_id, compressed_size, uncompressed_size });
        };

        let chunk_offset_parts = ["unknown1", "unknown2", "chunk_id", "compressed_size", "uncompressed_size"].to_vec();
        parse_vector(input, start_offset + chunks_offset as u32, &chunk_offset_parts, &mut parse_single_chunk);

        bundles.push(Bundle { bundle_id, chunks: chunks_list });
    };

    let bundle_offset_parts = ["bundle_id", "chunks", "unknown", "header_size"].to_vec();
    parse_vector(input, bundles_start, &bundle_offset_parts, &mut parse_single_bundle);

    bundles
}

fn languages(input: &[u8], languages_start: u32) -> Vec<Language> {
    let mut languages = Vec::<Language>::new();

    let mut parse_single_language = |start_offset: u32, entry_offsets: HashMap<String, u16>| {
        let name_offset = entry_offsets.get("name_offset").unwrap().to_owned() as u32;
        let name_position = crate::parse_single!(le_u32, &input[(start_offset + name_offset as u32) as usize..]);
        let name_data_offset = start_offset + name_offset + name_position;
        let name_length = crate::parse_single!(le_u32, &input[name_data_offset as usize..]);
        let name =
            String::from_utf8_lossy(&input[(name_data_offset + 4) as usize..(name_data_offset + 4 + name_length) as usize]).into_owned();

        let language_id_offset = entry_offsets.get("language_id").unwrap().to_owned();
        let language_id = crate::parse_single!(le_u8, &input[(start_offset + language_id_offset as u32) as usize..]);

        languages.push(Language { id: language_id, name });
    };

    let languages_offset_parts = ["name_offset", "unknown1", "language_id"].to_vec();
    parse_vector(input, languages_start, &languages_offset_parts, &mut parse_single_language);

    languages
}

fn directories(input: &[u8], directories_start: u32) -> Vec<Directory> {
    let mut directories = Vec::<Directory>::new();

    let mut parse_single_directory = |start_offset: u32, entry_offsets: HashMap<String, u16>| {
        let name_offset = entry_offsets.get("name_offset").unwrap().to_owned() as u32;
        let name_position = crate::parse_single!(le_u32, &input[(start_offset + name_offset as u32) as usize..]);
        let name_data_offset = start_offset + name_offset + name_position;
        let name_length = crate::parse_single!(le_u32, &input[name_data_offset as usize..]);
        let name =
            String::from_utf8_lossy(&input[(name_data_offset + 4) as usize..(name_data_offset + 4 + name_length) as usize]).into_owned();

        let directory_id_offset = entry_offsets.get("directory_id").unwrap().to_owned();
        let directory_id = if directory_id_offset > 0 {
            crate::parse_single!(le_u64, &input[(start_offset + directory_id_offset as u32) as usize..])
        } else {
            0
        };

        let parent_id_offset = entry_offsets.get("parent_id").unwrap().to_owned();
        let parent_id = if parent_id_offset > 0 {
            crate::parse_single!(le_u64, &input[(start_offset + parent_id_offset as u32) as usize..])
        } else {
            0
        };

        directories.push(Directory { id: directory_id, name, parent_id });
    };

    let directory_offset_parts = ["unknown1", "unknown2", "directory_id", "parent_id", "name_offset"].to_vec();
    parse_vector(input, directories_start, &directory_offset_parts, &mut parse_single_directory);

    directories
}

fn files(input: &[u8], files_start: u32) -> Vec<FileEntry> {
    let mut files = Vec::<FileEntry>::new();

    let mut parse_single_file = |start_offset: u32, entry_offsets: HashMap<String, u16>| {
        let name_offset = entry_offsets.get("name_offset").unwrap().to_owned() as u32;
        let name_position = crate::parse_single!(le_u32, &input[(start_offset + name_offset as u32) as usize..]);
        let name_data_offset = start_offset + name_offset + name_position;
        let name_length = crate::parse_single!(le_u32, &input[name_data_offset as usize..]);
        let name =
            String::from_utf8_lossy(&input[(name_data_offset + 4) as usize..(name_data_offset + 4 + name_length) as usize]).into_owned();

        let symlink_offset = entry_offsets.get("symlink_offset").unwrap().to_owned() as u32;
        let symlink_position = crate::parse_single!(le_u32, &input[(start_offset + symlink_offset as u32) as usize..]);
        let symlink_data_offset = start_offset + symlink_offset + symlink_position;
        let symlink_length = crate::parse_single!(le_u32, &input[symlink_data_offset as usize..]);
        let symlink =
            String::from_utf8_lossy(&input[(symlink_data_offset + 4) as usize..(symlink_data_offset + 4 + symlink_length) as usize])
                .into_owned();

        let file_id_offset = entry_offsets.get("file_id").unwrap().to_owned() as u32;
        let file_id = crate::parse_single!(le_u64, &input[(start_offset + file_id_offset as u32) as usize..]);

        let directory_id_offset = entry_offsets.get("directory_id").unwrap().to_owned() as u32;
        let directory_id = crate::parse_single!(le_u64, &input[(start_offset + directory_id_offset as u32) as usize..]);

        let file_size_id_offset = entry_offsets.get("file_size").unwrap().to_owned() as u32;
        let file_size = crate::parse_single!(le_u32, &input[(start_offset + file_size_id_offset as u32) as usize..]);

        let language_mask_offset = entry_offsets.get("language_mask").unwrap().to_owned() as u32;
        let language_mask = crate::parse_single!(le_u32, &input[(start_offset + language_mask_offset as u32) as usize..]);

        let mut chunks = Vec::<u64>::new();
        let chunks_offset = start_offset + entry_offsets.get("chunks").unwrap().to_owned() as u32;

        let mut append_to_chunks = |v: u64| {
            chunks.push(v);
        };

        parse_long_vector(input, chunks_offset, &mut append_to_chunks);

        files.push(FileEntry { id: file_id, name, symlink, directory_id, language: language_mask, size: file_size, chunk_ids: chunks })
    };

    let files_offset_parts = [
        "unknown1",
        "chunks",
        "file_id",
        "directory_id",
        "file_size",
        "name_offset",
        "language_mask",
        "unknown2",
        "unknown3",
        "unknown4",
        "unknown5",
        "symlink_offset",
        "unknown6",
        "unknown7",
        "unknown8",
    ]
    .to_vec();

    parse_vector(input, files_start, &files_offset_parts, &mut parse_single_file);

    files
}
