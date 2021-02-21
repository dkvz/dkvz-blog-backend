/* 
 * The stats module is meant to group 
 * all the GeoIP and anonymous stats 
 * systems together.
 */

use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};
use color_eyre::Result;
use eyre::{WrapErr, eyre};
use crate::db::{Pool, Connection, insert_article_stat};
use crate::db::entities::ArticleStat;

mod pseudonymizer;
pub mod ip_location;

enum StatsMessage {
  Close,
  InsertArticleStats(ArticleStat)
}

pub struct StatsService {
  tx: Sender<StatsMessage>,
  thread_handle: Option<JoinHandle<()>>
}

impl StatsService {

  pub fn open(
    pool: &Pool
  ) -> Result<StatsService> 
  {
    let (tx, rx) = mpsc::channel::<StatsMessage>();
    let connection = pool.clone().get()?;
    let thread_handle = thread::spawn(move || loop {
      match rx.recv() {
        Ok(msg) => {
          match msg {
            StatsMessage::Close => {
              println!("Stats thread terminating...");
              break;
            },
            StatsMessage::InsertArticleStats(article_stat) => {
              if let Err(e) = insert_article_stat(&connection, &article_stat) {
                eprintln!("Error from StatsService: \
                  could not insert ArticleStats - {}", e);
              }
            }
          }
        },
        // Stop the stat thread in case of error:
        Err(_) => break
      }
    });
    Ok(StatsService {
      tx,
      thread_handle: Some(thread_handle)
    })
  }

  // TODO We have to check if thread is alive before
  // using it. Return error immediately if not.
  pub fn insert_article_stats(
    article_stats: &ArticleStat
  ) -> Result<()> {
    
  }

}

// Not sure that'll work but Drop is a good place to ask for 
// termination of the thread. Which should be alive so we 
// don't check for that either.
impl Drop for StatsService {
  fn drop(&mut self) {
    match self.tx.clone().send(StatsMessage::Close) {
      Ok(_) => println!("StatsService is closing..."),
      Err(e) => eprintln!("Could not close StatsService - {}", e)
    }
    // I would have waited for the thread_handle to join here
    // but you can't. Something about it already being dropped
    // or uh... Yeah I don't know.
    // Then I used an Option for the thread handler and it 
    // worked if I use this really weird code.
    // Thanks Stackoverflow.
    self.thread_handle.take().map(JoinHandle::join);
  }
}