/* 
 * The stats module is meant to group 
 * all the GeoIP and anonymous stats 
 * systems together.
 */

use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};
use color_eyre::Result;
use eyre::{WrapErr, eyre};
use std::net::IpAddr;
use crate::db::{Pool, insert_article_stat};
use crate::db::entities::ArticleStat;
use crate::utils::text_utils::first_letter_to_upper;
pub mod pseudonymizer;
pub mod ip_location;
use ip_location::{IpLocator, GeoInfo};
use pseudonymizer::WordlistPseudoyimizer;

pub struct BaseArticleStat {
  pub article_id: i32,
  pub client_ua: String,
  pub client_ip: IpAddr
}

enum StatsMessage {
  Close,
  InsertArticleStats(BaseArticleStat)
}

pub struct StatsService {
  tx: Sender<StatsMessage>,
  thread_handle: Option<JoinHandle<()>>
}

// TODO Add a way to generate the ArticleStats object from 
// StatsService, could be a static method.
/*let mut ip_locator = IpLocator::open(&config.iploc_path)?;
  println!("{:?}", ip_locator.geo_info("127.0.0.1"));*/

impl StatsService {

  pub fn open(
    pool: &Pool,
    wordlist_path: &str,
    iploc_path: &str
  ) -> Result<StatsService> 
  {
    let mut pseudonymizer = WordlistPseudoyimizer::open(wordlist_path)?;
    let mut ip_locator = IpLocator::open(iploc_path)?;
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
            StatsMessage::InsertArticleStats(base_article_stat) => {
              let client_ip = base_article_stat.client_ip.to_string();
              // Get the geoip info:
              let geo_info = match ip_locator.geo_info(&client_ip) {
                Ok(info) => info,
                Err(e) => {
                  eprintln!("Error from StatsService for IP Location \
                    - {}", e);
                  GeoInfo {
                    country: String::new(),
                    region: String::new(),
                    city: String::new()
                  }
                }
              };
              let article_stat = ArticleStat {
                id: -1,
                article_id: base_article_stat.article_id,
                pseudo_ua: pseudonymize(&mut pseudonymizer, &base_article_stat.client_ua),
                pseudo_ip: pseudonymize(&mut pseudonymizer, &client_ip),
                client_ua: base_article_stat.client_ua,
                client_ip,
                date: None,
                country: geo_info.country,
                region: geo_info.region,
                city: geo_info.city
              };
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

  pub fn insert_article_stats(
    &self,
    article_stats: BaseArticleStat
  ) -> Result<()> {
    // The message sending will fail if the thread is dead.
    // I could make everything panic in that case but I 
    // won't.
    let tx = self.tx.clone();
    tx.send(StatsMessage::InsertArticleStats(article_stats))
      .context("Send article stats to stats thread")
  }

}

fn pseudonymize(
  pseudonymizer: &mut WordlistPseudoyimizer,
  value: &str
) -> String {
  // We just return an empty string if the pseudonimizer 
  // doesn't work for some reason, but we log the error.
  match pseudonymizer.pseudonymize(value) {
    Ok(pseudo) => first_letter_to_upper(pseudo),
    Err(e) => {
      eprintln!("Error - Could not pseudonymize value - {}", e);
      String::new()
    }
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