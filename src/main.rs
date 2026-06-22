//! `enxame` — the ENXAME BitTorrent command-line client.
//!
//! Inspection + control front for the suite: read a `.torrent` or a
//! `magnet:` link and report its identity, files, and trackers. The
//! actual transfer is driven by `enxamed` (the daemon); this is the
//! ergonomic CLI that the GUIs mirror. Typed-emission throughout —
//! output is assembled from typed values, never ad-hoc string soup.

use std::process::ExitCode;

use enxame_magnet::{Magnet, Xt};
use enxame_metainfo::{Layout, Metainfo};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else {
        usage();
        return ExitCode::FAILURE;
    };
    let result = match cmd.as_str() {
        "info" => match args.next() {
            Some(path) => cmd_info(&path),
            None => Err("usage: enxame info <file.torrent>".into()),
        },
        "magnet" => match args.next() {
            Some(uri) => cmd_magnet(&uri),
            None => Err("usage: enxame magnet <magnet:?...>".into()),
        },
        "-h" | "--help" | "help" => {
            usage();
            Ok(())
        }
        other => Err(format!("unknown command `{other}` (try `enxame help`)")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("enxame: {e}");
            ExitCode::FAILURE
        }
    }
}

fn usage() {
    eprintln!(
        "enxame — pleme-io BitTorrent client\n\
         \n\
         USAGE:\n  \
         enxame info <file.torrent>   inspect a torrent (name, files, info-hash, trackers)\n  \
         enxame magnet <magnet:?...>  inspect a magnet link\n\
         \n\
         The transfer daemon is `enxamed <file.torrent> [out-dir]`."
    );
}

fn cmd_info(path: &str) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("reading {path}: {e}"))?;
    let m = Metainfo::from_bytes(&bytes).map_err(|e| format!("parsing torrent: {e}"))?;

    println!("name        {}", m.info.name);
    println!("info-hash   {}", m.info_hash);
    println!("pieces      {} × {} bytes", m.info.piece_count(), m.info.piece_length);
    println!("total       {} bytes", m.info.total_length());
    println!("private     {}", m.info.private);
    if let Some(announce) = &m.announce {
        println!("tracker     {announce}");
    }
    for tier in &m.announce_list {
        for url in tier {
            println!("tracker     {url}");
        }
    }
    if let Some(c) = &m.comment {
        println!("comment     {c}");
    }
    match &m.info.layout {
        Layout::SingleFile { length } => println!("file        {} ({length} bytes)", m.info.name),
        Layout::MultiFile { files } => {
            println!("files       {}", files.len());
            for f in files {
                println!("  {:>12}  {}", f.length, f.path.join("/"));
            }
        }
    }
    Ok(())
}

fn cmd_magnet(uri: &str) -> Result<(), String> {
    let magnet: Magnet = uri.parse().map_err(|e| format!("parsing magnet: {e}"))?;
    if let Some(name) = &magnet.display_name {
        println!("name        {name}");
    }
    for xt in &magnet.topics {
        match xt {
            Xt::BtihV1(h) => {
                let mut hex = String::with_capacity(40);
                for b in h {
                    hex.push_str(&byte_hex(*b));
                }
                println!("info-hash   {hex} (v1)");
            }
            Xt::BtmhV2(_) => println!("info-hash   <btmh v2 multihash>"),
        }
    }
    for tr in &magnet.trackers {
        println!("tracker     {tr}");
    }
    for p in &magnet.peers {
        println!("peer        {p}");
    }
    Ok(())
}

fn byte_hex(b: u8) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(2);
    s.push(HEX[(b >> 4) as usize] as char);
    s.push(HEX[(b & 0x0f) as usize] as char);
    s
}
