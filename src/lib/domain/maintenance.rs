use tokio::runtime::Handle;
use crate::data::DatabasePool;
use crate::service;

pub struct Maintenance;

impl Maintenance {
    pub fn spawn(pool: DatabasePool, handle: Handle) -> Self {
        handle.spawn(async move {
           let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
           loop {
               interval.tick().await;
               if let Err(e) = service::action::delete_expires(&pool).await {
                   eprintln!("Error cleaning up expired clips: {}", e);
               }
           }
        });
        Self
    }
}
