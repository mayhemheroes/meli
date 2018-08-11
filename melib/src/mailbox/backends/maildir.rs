/*
 * meli - mailbox module.
 *
 * Copyright 2017 Manos Pitsidianakis
 *
 * This file is part of meli.
 *
 * meli is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * meli is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with meli. If not, see <http://www.gnu.org/licenses/>.
 */

use async::*;
use conf::AccountSettings;
use error::{MeliError, Result};
use mailbox::backends::{
    BackendFolder, BackendOp, BackendOpGenerator, Folder, MailBackend, RefreshEvent,
    RefreshEventConsumer,
};
use mailbox::email::parser;
use mailbox::email::{Envelope, Flag};

extern crate notify;

use self::notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::time::Duration;

use std::sync::mpsc::channel;
//use std::sync::mpsc::sync_channel;
//use std::sync::mpsc::SyncSender;
//use std::time::Duration;
use std::thread;
extern crate crossbeam;
use memmap::{Mmap, Protection};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

/// `BackendOp` implementor for Maildir
#[derive(Debug, Default)]
pub struct MaildirOp {
    path: String,
    slice: Option<Mmap>,
}

impl Clone for MaildirOp {
    fn clone(&self) -> Self {
        MaildirOp {
            path: self.path.clone(),
            slice: None,
        }
    }
}

impl MaildirOp {
    pub fn new(path: String) -> Self {
        MaildirOp {
            path: path,
            slice: None,
        }
    }
}

impl BackendOp for MaildirOp {
    fn description(&self) -> String {
        format!("Path of file: {}", self.path)
    }
    fn as_bytes(&mut self) -> Result<&[u8]> {
        if self.slice.is_none() {
            self.slice = Some(Mmap::open_path(self.path.to_string(), Protection::Read)?);
        }
        /* Unwrap is safe since we use ? above. */
        Ok(unsafe { self.slice.as_ref().unwrap().as_slice() })
    }
    fn fetch_headers(&mut self) -> Result<&[u8]> {
        let raw = self.as_bytes()?;
        let result = parser::headers_raw(raw).to_full_result()?;
        Ok(result)
    }
    fn fetch_body(&mut self) -> Result<&[u8]> {
        let raw = self.as_bytes()?;
        let result = parser::headers_raw(raw).to_full_result()?;
        Ok(result)
    }
    fn fetch_flags(&self) -> Flag {
        let mut flag = Flag::default();
        let path = PathBuf::from(&self.path);
        let filename = path.file_name().unwrap().to_str().unwrap();
        if !filename.contains(":2,") {
            return flag;
        }

        for f in filename.chars().rev() {
            match f {
                ',' => break,
                'D' => flag |= Flag::DRAFT,
                'F' => flag |= Flag::FLAGGED,
                'P' => flag |= Flag::PASSED,
                'R' => flag |= Flag::REPLIED,
                'S' => flag |= Flag::SEEN,
                'T' => flag |= Flag::TRASHED,
                _ => panic!(),
            }
        }

        flag
    }
    fn set_flag(&mut self, envelope: &mut Envelope, f: &Flag) -> Result<()> {
        let idx: usize = self.path.rfind(":2,").ok_or(MeliError::new(format!(
            "Invalid email filename: {:?}",
            self
        )))? + 3;
        let mut new_name: String = self.path[..idx].to_string();
        let mut flags = self.fetch_flags();
        flags.toggle(*f);
        if !(flags & Flag::DRAFT).is_empty() {
            new_name.push('D');
        }
        if !(flags & Flag::FLAGGED).is_empty() {
            new_name.push('F');
        }
        if !(flags & Flag::PASSED).is_empty() {
            new_name.push('P');
        }
        if !(flags & Flag::REPLIED).is_empty() {
            new_name.push('R');
        }
        if !(flags & Flag::SEEN).is_empty() {
            new_name.push('S');
        }
        if !(flags & Flag::TRASHED).is_empty() {
            new_name.push('T');
        }

        fs::rename(&self.path, &new_name)?;
        envelope.set_operation_token(Box::new(BackendOpGenerator::new(Box::new(move || {
            Box::new(MaildirOp::new(new_name.clone()))
        }))));
        Ok(())
    }
}

/// Maildir backend https://cr.yp.to/proto/maildir.html
#[derive(Debug)]
pub struct MaildirType {
    folders: Vec<MaildirFolder>,
    path: String,
}

impl MailBackend for MaildirType {
    fn folders(&self) -> Vec<Folder> {
        self.folders.iter().map(|f| f.clone()).collect()
    }
    fn get(&self, folder: &Folder) -> Async<Result<Vec<Envelope>>> {
        self.multicore(4, folder)
    }
    fn watch(&self, sender: RefreshEventConsumer) -> Result<()> {
        let (tx, rx) = channel();
        let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();
        for f in &self.folders {
            if f.is_valid().is_err() {
                continue;
            }
            eprintln!("watching {:?}", f);
            let mut p = PathBuf::from(&f.path);
            p.push("cur");
            watcher.watch(&p, RecursiveMode::NonRecursive).unwrap();
            p.pop();
            p.push("new");
            watcher.watch(&p, RecursiveMode::NonRecursive).unwrap();
        }
        thread::Builder::new()
            .name("folder watch".to_string())
            .spawn(move || {
                // Move `watcher` in the closure's scope so that it doesn't get dropped.
                let _watcher = watcher;
                loop {
                    match rx.recv() {
                        Ok(event) => match event {
                            DebouncedEvent::Create(mut pathbuf)
                            | DebouncedEvent::Remove(mut pathbuf) => {
                                let path = if pathbuf.is_dir() {
                                    if pathbuf.ends_with("cur") | pathbuf.ends_with("new") {
                                        pathbuf.pop();
                                    }
                                    pathbuf.to_str().unwrap()
                                } else {
                                    pathbuf.pop();
                                    pathbuf.parent().unwrap().to_str().unwrap()
                                };
                                eprintln!(" got event in {}", path);

                                let mut hasher = DefaultHasher::new();
                                hasher.write(path.as_bytes());
                                sender.send(RefreshEvent {
                                    folder: format!("{}", path),
                                    hash: hasher.finish(),
                                });
                            }
                            _ => {}
                        },
                        Err(e) => eprintln!("watch error: {:?}", e),
                    }
                }
            })?;
        Ok(())
    }
}

impl MaildirType {
    pub fn new(f: &AccountSettings) -> Self {
        let mut folders: Vec<MaildirFolder> = Vec::new();
        fn recurse_folders<P: AsRef<Path>>(folders: &mut Vec<MaildirFolder>, p: P) -> Vec<usize> {
            let mut children = Vec::new();
            for mut f in fs::read_dir(p).unwrap() {
                for f in f.iter_mut() {
                    {
                        let path = f.path();
                        if path.ends_with("cur") || path.ends_with("new") || path.ends_with("tmp") {
                            continue;
                        }
                        if path.is_dir() {
                            let path_children = recurse_folders(folders, &path);
                            if let Ok(f) = MaildirFolder::new(
                                path.to_str().unwrap().to_string(),
                                path.file_name().unwrap().to_str().unwrap().to_string(),
                                path_children,
                            ) {
                                folders.push(f);
                                children.push(folders.len() - 1);
                            }
                        }
                    }
                }
            }
            children
        };
        let path = PathBuf::from(f.root_folder());
        let path_children = recurse_folders(&mut folders, &path);
        if path.is_dir() {
            if let Ok(f) = MaildirFolder::new(
                path.to_str().unwrap().to_string(),
                path.file_name().unwrap().to_str().unwrap().to_string(),
                path_children,
            ) {
                folders.push(f);
            }
        }
        MaildirType {
            folders,
            path: f.root_folder().to_string(),
        }
    }
    fn owned_folder_idx(&self, folder: &Folder) -> usize {
        for (idx, f) in self.folders.iter().enumerate() {
            if f.hash() == folder.hash() {
                return idx;
            }
        }
        unreachable!()
    }

    pub fn multicore(&self, cores: usize, folder: &Folder) -> Async<Result<Vec<Envelope>>> {
        let mut w = AsyncBuilder::new();
        let handle = {
            let tx = w.tx();
            // TODO: Avoid clone
            let folder: &MaildirFolder = &self.folders[self.owned_folder_idx(folder)];
            let path = folder.path().to_string();
            let name = format!("parsing {:?}", folder.name());

            thread::Builder::new()
                .name(name)
                .spawn(move || {
                    let mut path = PathBuf::from(path);
                    path.push("cur");
                    let iter = path.read_dir()?;
                    let count = path.read_dir()?.count();
                    let mut files: Vec<String> = Vec::with_capacity(count);
                    let mut r = Vec::with_capacity(count);
                    for e in iter {
                        let e = e.and_then(|x| {
                            let path = x.path();
                            Ok(path.to_str().unwrap().to_string())
                        })?;
                        files.push(e);
                    }
                    let mut threads = Vec::with_capacity(cores);
                    if !files.is_empty() {
                        crossbeam::scope(|scope| {
                            let chunk_size = if count / cores > 0 {
                                count / cores
                            } else {
                                count
                            };
                            for chunk in files.chunks(chunk_size) {
                                let mut tx = tx.clone();
                                let s = scope.spawn(move || {
                                    let len = chunk.len();
                                    let size = if len <= 100 { 100 } else { (len / 100) * 100 };
                                    let mut local_r: Vec<
                                        Envelope,
                                    > = Vec::with_capacity(chunk.len());
                                    for c in chunk.chunks(size) {
                                        let len = c.len();
                                        for e in c {
                                            let e_copy = e.to_string();
                                            if let Some(mut e) = Envelope::from_token(Box::new(
                                                BackendOpGenerator::new(Box::new(move || {
                                                    Box::new(MaildirOp::new(e_copy.clone()))
                                                })),
                                            )) {
                                                if e.populate_headers().is_err() {
                                                    continue;
                                                }
                                                local_r.push(e);
                                            }
                                        }
                                        tx.send(AsyncStatus::ProgressReport(len));
                                    }
                                    local_r
                                });
                                threads.push(s);
                            }
                        });
                    }
                    for t in threads {
                        let mut result = t.join();
                        r.append(&mut result);
                    }
                    tx.send(AsyncStatus::Finished);
                    Ok(r)
                })
                .unwrap()
        };
        w.build(handle)
    }
}

#[derive(Debug, Default)]
pub struct MaildirFolder {
    hash: u64,
    name: String,
    path: String,
    children: Vec<usize>,
}

impl MaildirFolder {
    pub fn new(path: String, file_name: String, children: Vec<usize>) -> Result<Self> {
        let mut h = DefaultHasher::new();
        h.write(&path.as_bytes());

        let ret = MaildirFolder {
            hash: h.finish(),
            name: file_name,
            path: path,
            children: children,
        };
        ret.is_valid()?;
        Ok(ret)
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    fn is_valid(&self) -> Result<()> {
        let path = self.path();
        let mut p = PathBuf::from(path);
        for d in &["cur", "new", "tmp"] {
            p.push(d);
            if !p.is_dir() {
                return Err(MeliError::new(format!(
                    "{} is not a valid maildir folder",
                    path
                )));
            }
            p.pop();
        }
        Ok(())
    }
}
impl BackendFolder for MaildirFolder {
    fn hash(&self) -> u64 {
        self.hash
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn children(&self) -> &Vec<usize> {
        &self.children
    }
    fn clone(&self) -> Folder {
        Box::new(MaildirFolder {
            hash: self.hash,
            name: self.name.clone(),
            path: self.path.clone(),
            children: self.children.clone(),
        })
    }
}
