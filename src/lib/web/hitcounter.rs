use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Handle;
use crate::{service, ServiceError, ShortCode};
use crossbeam_channel::{unbounded, Sender, Receiver, TryRecvError};
use parking_lot::Mutex;
use crate::data::DatabasePool;

type HitStore = Arc<Mutex<HashMap<ShortCode, u32>>>;

#[derive(Debug, thiserror::Error)]
enum HitCountError {
    #[error("service error: {0}")]
    Service(#[from] ServiceError),
    #[error("communication error: {0}")]
    Channel(#[from] crossbeam_channel::SendError<HitCountMsg>),
}

enum HitCountMsg {
    Commit,
    Hit(ShortCode, u32),
}

pub struct HitCounter {
    tx: Sender<HitCountMsg>,
}

impl HitCounter {

    fn commit_hits(hits: HitStore, handle: Handle, pool: DatabasePool) -> Result<(), HitCountError> {
        let hits = Arc::clone(&hits);
        let hits: Vec<(ShortCode, u32)> = {
            let mut hits = hits.lock();
            let hits_vec = hits.iter().map(|(k, v)| (k.clone(), *v)).collect();
            hits.clear();
            hits_vec
        };

        handle.block_on(async move {
            let transaction = service::action::begin_transaction(&pool).await?;
            for (shortcode, hits) in hits {
                if let Err(e) = service::action::increase_hit_count(&shortcode, hits, &pool).await {
                    eprintln!("Error updating hit count: {}", e);
                }
            }
            Ok(service::action::end_transaction(transaction).await?)
        })
    }

    fn process_msg(msg: HitCountMsg, hits: HitStore, handle: Handle, pool: DatabasePool) -> Result<(), HitCountError> {
        match msg {
            HitCountMsg::Commit => Self::commit_hits(hits.clone(), handle.clone(), pool.clone())?,
            HitCountMsg::Hit(shortcode, count) => {
                let mut hitcount = hits.lock();
                let hitcount = hitcount.entry(shortcode).or_insert(0);
                *hitcount += count;
            }
        }
        Ok(())
    }
    pub fn new(pool: DatabasePool, handle: Handle) -> Self {
       let (tx, rx) = unbounded();
       let tx_clone = tx.clone();
       let rx_clone = rx.clone();

        let _ = std::thread::spawn(move || {
           println!("Hit counter thread spawned");
           let store: HitStore = Arc::new(Mutex::new(HashMap::new()));

            loop {
                match rx_clone.try_recv() {
                    Ok(msg) => if let Err(e) = Self::process_msg(msg, store.clone(), handle.clone(), pool.clone()) {
                        eprintln!("Error processing hit count message: {}", e);
                    },
                    Err(e) => match e {
                        TryRecvError::Empty => {
                            std::thread::sleep(std::time::Duration::from_secs(5));
                            if let Err(e) = tx_clone.send(HitCountMsg::Commit) {
                                eprintln!("Error sending commit message: {}", e);
                            }
                        }
                        _ => break
                    }
                }
            }
        });

        Self { tx }
    }

    pub async fn hit(&self, shortcode: ShortCode, hits: u32) {
       if let Err(e) = self.tx.send(HitCountMsg::Hit(shortcode, hits)) {
           eprintln!("Error sending hit count message: {}", e);
       }
    }

}