#[macro_use]

extern crate log;
extern crate env_logger;
extern crate libc;
extern crate nix;
extern crate time;

use fuse::*;
use std::collections::*;
use std::env;
use std::ffi::OsStr;
use std::process::Command;
mod fs;

fn main() {
    env_logger::init();
    info!("logger init");
    let mountpoint = env::args().nth(1).expect("usage: rust_fuse MOUNTPOINT");
    let mut inodes = HashMap::new();

    // ルートディレクトリを作っておく
    inodes.insert(
        1,
        (
            0,
            "/".to_string(),
            fs::file_create(1, 0, FileType::Directory),
        ),
    );

    let path = mountpoint.clone();
    ctrlc::set_handler(move || {
        Command::new("umount")
            .arg("-f")
            .arg(&path)
            .output()
            .expect("failed to execute process");
    })
    .expect("error setting Ctrl-C handler");

    fuse::mount(
        fs::MemoryFS {
            inodes: inodes,
            datas: HashMap::new(),
        },
        &OsStr::new(&mountpoint),
        &[],
    )
    .expect("fail mount()");
}
