use crate::data::Card;
use crate::error::RuntimeError;
use crate::logs::LogMsg;
use crate::pipeline::Visitor;
use crate::template::Template;
use md5::{Digest, Md5};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};

pub trait DiffHash {
    fn diff_hash(&self, state: &mut Md5);
}

#[derive(Clone)]
pub struct DiffVisitor {
    tx: Option<Sender<LogMsg>>,
    hasher: Arc<RwLock<DiffHasher>>,
}

struct DiffHasher {
    fp: PathBuf,
    data: HashMap<u64, [u8; 16]>,
    new_data: HashMap<u64, [u8; 16]>,
}

impl DiffHasher {
    fn read(fp: PathBuf) -> Self {
        let mut data = HashMap::new();
        let _ = File::open(&fp).map(|mut file| {
            while let Ok((k, v)) = Self::read_line(&mut file) {
                data.insert(k, v);
            }
        });
        let new_data = HashMap::new();
        Self { fp, data, new_data }
    }

    fn read_line(file: &mut File) -> std::io::Result<(u64, [u8; 16])> {
        let mut key_buffer = [0u8; 8];
        let mut val_buffer = [0u8; 16];
        file.read_exact(&mut key_buffer)?;
        let key = u64::from_be_bytes(key_buffer);
        file.read_exact(&mut val_buffer)?;
        Ok((key, val_buffer))
    }

    fn write(&self) -> std::io::Result<()> {
        let mut file = File::create(&self.fp)?;
        let pairs = self
            .data
            .iter()
            .filter(|(k, _)| !self.new_data.contains_key(k))
            .chain(self.new_data.iter());
        for (k, v) in pairs {
            file.write(&k.to_be_bytes())?;
            file.write(v)?;
        }
        Ok(())
    }
}

impl DiffVisitor {
    pub fn new(tx: Option<Sender<LogMsg>>, fp: PathBuf) -> Self {
        let msg = format!("reading diff digest from {}", fp.display());
        let hasher = DiffHasher::read(fp);
        let _ = tx.as_ref().map(|tx| tx.send(LogMsg::Info(0, msg)));
        let hasher = Arc::new(RwLock::new(hasher));
        Self { tx, hasher }
    }

    fn on_read_internal<C, T>(&self, template: &T, card: &Result<C, Box<dyn Error>>) -> Option<bool>
    where
        C: Card + DiffHash + std::fmt::Debug,
        T: Template<C>,
    {
        // TODO: check artwork, assets, fonts... ?
        let card = card.as_ref().ok()?;
        let id_hash = {
            let id = template.identify(card);
            let mut id_hasher = DefaultHasher::new();
            id.hash(&mut id_hasher);
            id_hasher.finish()
        };
        let new_digest: [u8; 16] = {
            let mut hasher = Md5::new();
            card.diff_hash(&mut hasher);
            hasher.finalize().as_slice().try_into().unwrap()
        };
        let mut diff_hasher = self.hasher.write().ok()?;
        let old_digest = diff_hasher.data.get(&id_hash)?;
        if old_digest != &new_digest {
            diff_hasher.new_data.insert(id_hash, new_digest);
            Some(true)
        } else {
            Some(false)
        }
    }
}

impl<C, T> Visitor<C, T> for DiffVisitor
where
    C: Card + DiffHash + std::fmt::Debug,
    T: Template<C>,
{
    fn on_read(&self, template: &T, card: &Result<C, Box<dyn Error>>) -> bool {
        self.on_read_internal(template, card).unwrap_or(true)
    }

    fn on_iter_err_r(&self, template: &T, _worker: usize, _i: usize, card: &C, _error: &RuntimeError) {
        let id_hash = {
            let id = template.identify(card);
            let mut id_hasher = DefaultHasher::new();
            id.hash(&mut id_hasher);
            id_hasher.finish()
        };
        if let Ok(mut diff_hasher) = self.hasher.write() {
            diff_hasher.data.remove(&id_hash);
            diff_hasher.new_data.remove(&id_hash);
        }
    }

    fn on_finish(&self, _template: &T, worker: usize, _result: &Result<(), RuntimeError>) {
        if worker != 0 {
            return;
        }
        if let Ok(diff_hasher) = self.hasher.read() {
            let _ = match diff_hasher.write() {
                Ok(()) => self.tx.as_ref().map(|tx| {
                    tx.send(LogMsg::Info(
                        0,
                        format!("saved diff digest to {}", diff_hasher.fp.display()),
                    ))
                }),
                Err(e) => self.tx.as_ref().map(|tx| {
                    tx.send(LogMsg::Warn(0, format!("failed to write diff digest: {e}")))
                }),
            };
        }
    }
}
