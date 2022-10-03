// SPDX-License-Identifier: MIT License
// Copyright (C) 2020 Advanced Micro Devices, Inc. 
//
// amd\_ucode\_info\_rs is my personal project porting Python to Rust.
// Copyright (c) 2022 Umio Yasuno

use std::io;
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::fs;
use std::fs::File;
use std::path::Path;
use std::collections::HashMap;

mod opt;
use opt::MainOpt;

const EQ_TABLE_ENTRY_SIZE: u64 = 16;
const EQ_TABLE_LEN_OFFSET: u64 = 8;
const EQ_TABLE_OFFSET: u64 = 12;

fn read_u16(ucode_file: &mut File) -> io::Result<u16> {
    let mut buf = [0u8; 2];
    ucode_file.read(&mut buf[..])?;

    Ok(u16::from_le_bytes(buf))
}

fn read_u32(ucode_file: &mut File) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    ucode_file.read(&mut buf[..])?;

    Ok(u32::from_le_bytes(buf))
}

fn parse_equiv_table(ucode_file: &mut File, table_len: u64) -> io::Result<HashMap<u16, u32>> {
    let mut table = HashMap::new();

    let mut table_item = EQ_TABLE_OFFSET;
    let table_stop = EQ_TABLE_OFFSET + table_len;

    while table_item < table_stop {
        ucode_file.seek(SeekFrom::Start(table_item))?;

        let cpuid = read_u32(ucode_file)?;

        /* Skip errata mask and compare fields */
        ucode_file.seek(SeekFrom::Current(8))?;

        let equiv_id = read_u16(ucode_file)?;

        if equiv_id != 0 {
            table.insert(equiv_id, cpuid);
        }

        table_item += EQ_TABLE_ENTRY_SIZE;
    }

    Ok(table)
}

fn extract_patch(extract_dir: &String, patch_start: u64, patch_length: u32, ucode_file: &mut File, ucode_level: u32)
    -> io::Result<()> {
/*
    Extract raw microcode patch starting at patch_start to the directory
    provided by the -o option or the current directory if not specified.
    Directory will be created if it doesn't already exist.
*/
    if !Path::new(extract_dir).exists() {
        fs::create_dir(extract_dir)?;
    }

    let path = Path::new(extract_dir).join(format!("mc_patch_{ucode_level:#x}.bin"));

    ucode_file.seek(SeekFrom::Start(patch_start))?;

    let patch_bin = {
        let mut buf = vec![0u8; patch_length as usize];
        ucode_file.read(&mut buf[..])?;

        buf
    };

    let mut patch_file = File::create(&path)?;
    patch_file.write_all(&patch_bin)?;

    println!("    Patch extracted to {}", &path.display());

    Ok(())
}

/* The cpuid is the equivalent to CPUID_Fn00000001_EAX */
fn fms(cpuid: u32) -> (u32, u32, u32) {
    (
        ((cpuid >> 8) & 0xF) + ((cpuid >> 20) & 0xFF),
        ((cpuid >> 4) & 0xF) + ((cpuid >> 12) & 0xF0),
        cpuid & 0xF,
    )
}

fn main() -> io::Result<()> {
    let opt = MainOpt::parse();

    let extract = !opt.extract_dir.is_empty();
    let mut ucode_file = File::open(&opt.ucode_path)?;

    if let Some(file_name) = Path::new(&opt.ucode_path).file_name() {
        println!("Microcode patches in {file_name:?}:");
    }

    ucode_file.seek(SeekFrom::Start(0))?;

    /* Check magic number */
    {
        let mut buf = [0u8; 4];
        ucode_file.read(&mut buf[..])?;

        if &buf != b"DMA\x00" {
            eprintln!("ERROR: Missing magic number at beginning of container");
            return Err(io::Error::from(ErrorKind::InvalidInput));
        }
    }

    ucode_file.seek(SeekFrom::Start(EQ_TABLE_LEN_OFFSET))?;

    /* Read the equivalence table length */
    let table_len = {
        let mut buf = [0u8; 1];
        ucode_file.read(&mut buf[..])?;

        buf[0] as u64
    };

    let table = parse_equiv_table(&mut ucode_file, table_len)?;

    let mut cursor = EQ_TABLE_OFFSET + table_len;
    let end_of_file = ucode_file.seek(SeekFrom::End(0))?;

    while cursor < end_of_file {
        /* Seek to the start of the patch information */
        ucode_file.seek(SeekFrom::Start(cursor))?;

        let patch_start = cursor + 8;
        let patch_type = read_u32(&mut ucode_file)?;

        if patch_type != 1 {
            eprintln!("Invalid patch identifier: {patch_type:#010X}");
            break;
        }

        let patch_length = read_u32(&mut ucode_file)?;

        ucode_file.seek(SeekFrom::Current(4))?;
        let ucode_level = read_u32(&mut ucode_file)?;

        ucode_file.seek(SeekFrom::Current(16))?;
        let equiv_id = read_u16(&mut ucode_file)?;

        let cpuid = match table.get(&equiv_id) {
            Some(cpuid) => cpuid,
            None => {
                eprintln!("Patch equivalence id not present in equivalence table ({equiv_id:#06X})");
                cursor += patch_length as u64 + 8;
                continue;
            },
        };

        let (family, model, stepping) = fms(*cpuid);

        println!("  Family={family:#04X}, Model={model:#04X}, Stepping={stepping:#04X}: Patch={ucode_level:#010X} Length={patch_length} bytes");

        if extract {
            extract_patch(&opt.extract_dir, patch_start, patch_length, &mut ucode_file, ucode_level)?;
        }

        cursor += patch_length as u64 + 8;
    }

    Ok(())
}
