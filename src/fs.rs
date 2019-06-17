use fuse::*;
use libc::*;
use std::collections::*;
use std::ffi::OsStr;
use std::os::raw::c_int;
use time::*;

const TTL: Timespec = Timespec { sec: 1, nsec: 0 };

pub fn file_create(ino: u64, size: u64, ftype: FileType) -> FileAttr {
  let t = time::now().to_timespec();
  FileAttr {
    ino: ino,
    size: size,
    blocks: 0,
    atime: t,
    mtime: t,
    ctime: t,
    crtime: t,
    kind: ftype,
    perm: match ftype {
      FileType::Directory => 0o755,
      _ => 0o644,
    },
    nlink: 2,
    uid: nix::unistd::getuid().as_raw(),
    gid: nix::unistd::getgid().as_raw(),
    rdev: 0,
    flags: 0,
  }
}

pub struct MemoryFS {
  // <ino, (parent_ino, path, fileattr)>
  pub inodes: HashMap<u64, (u64, String, FileAttr)>,
  // <ino, file_data>
  pub datas: HashMap<u64, String>,
}

impl Filesystem for MemoryFS {
  // 初期化
  fn init(&mut self, _req: &Request) -> Result<(), c_int> {
    info!("init");
    Ok(())
  }

  // GetAttr
  fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
    for (&inode, f) in self.inodes.iter() {
      if ino == inode {
        reply.attr(&TTL, &f.2);
        return;
      }
    }

    // 何も返すものがない場合はlibc::ENOENTを返しておく
    reply.error(ENOENT);
  }

  // ReadDir
  fn readdir(
    &mut self,
    _req: &Request,
    _ino: u64,
    _fh: u64,
    offset: i64,
    mut reply: ReplyDirectory,
  ) {
    if offset > 0 {
      reply.ok();
      return;
    }

    reply.add(1, 0, FileType::Directory, ".");
    reply.add(2, 1, FileType::Directory, "..");
    let mut reply_add_offset = 2;
    for (_, f) in self.inodes.iter() {
      if _ino == f.0 {
        let attr = f.2;
        let name = &f.1;
        reply.add(attr.ino, reply_add_offset, attr.kind, name);
        reply_add_offset += 1;
      }
    }
    reply.ok();
  }
  // Lookup
  fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
    for (_, f) in self.inodes.iter() {
      if f.0 == parent && name.to_str().unwrap() == f.1.as_str() {
        reply.entry(&TTL, &f.2, 0);
        return;
      }
    }
    reply.error(ENOENT);
  }
  // Create
  fn create(
    &mut self,
    _req: &Request,
    parent: u64,
    name: &OsStr,
    _mode: u32,
    _flag: u32,
    reply: ReplyCreate,
  ) {
    let inode = time::now().to_timespec().sec as u64;
    let f = file_create(inode, 0, FileType::RegularFile);
    self
      .inodes
      .insert(inode, (parent, name.to_str().unwrap().to_string(), f));
    reply.created(&TTL, &f, 0, 0, 0);
  }

  // SetAttr
  fn setattr(
    &mut self,
    _req: &Request,
    ino: u64,
    _mode: Option<u32>,
    _uid: Option<u32>,
    _gid: Option<u32>,
    _size: Option<u64>,
    _atime: Option<Timespec>,
    _mtime: Option<Timespec>,
    _fh: Option<u64>,
    _crtime: Option<Timespec>,
    _chgtime: Option<Timespec>,
    _bkuptime: Option<Timespec>,
    _flags: Option<u32>,
    reply: ReplyAttr,
  ) {
    match self.inodes.get(&ino) {
      Some(f) => reply.attr(&TTL, &f.2),
      None => reply.error(EACCES),
    }
  }
  // Write
  fn write(
    &mut self,
    _req: &Request,
    ino: u64,
    _fh: u64,
    _offset: i64,
    data: &[u8],
    _flags: u32,
    reply: ReplyWrite,
  ) {
    // 書き込み対象のデータ長を保持
    let length: usize = data.len();

    // Stringにしてdatasに格納する
    let x = String::from_utf8(data.to_vec()).expect("fail to-string");
    self.datas.insert(ino, x);
    if let Some(f) = self.inodes.get_mut(&ino) {
      let parent_ino = f.0;
      let name = f.1.clone();
      *f = (
        parent_ino,
        name,
        file_create(ino, length as u64, FileType::RegularFile),
      );
    }

    // 書き込みサイズを伝える
    reply.written(length as u32);
  }

  // Read
  fn read(
    &mut self,
    _req: &Request,
    ino: u64,
    _fh: u64,
    _offset: i64,
    _size: u32,
    reply: ReplyData,
  ) {
    match self.datas.get(&ino) {
      Some(x) => reply.data(x.as_bytes()), // 読み出しデータ内容を伝える
      None => reply.error(EACCES),         // 雑にlibc::EACCES返す
    }
  }

  // 終了処理
  fn destroy(&mut self, _req: &Request) {
    info!("destroy");
  }
}