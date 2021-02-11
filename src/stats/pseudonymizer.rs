use std::fs::File;
use std::io::prelude::*;
use std::collections::VecDeque;
use linecount::count_lines;
use color_eyre::Result;
use eyre::{WrapErr, eyre};

// Capacity of the queue I use for caching
const CACHE_CAPACITY: usize = 50;

pub struct WordlistPseudoyimizer {
  file: File,
  line_count: usize,
  cache: Cache
}

// I could more than one way to find lines in my 
// wordlist.
// https://docs.rs/indexed-line-reader/0.2.1/src/indexed_line_reader
// uses a binary tree created initially with all the lines position 
// as start and end bytes in the file. Kinda smart, do not know how
// much memory that represents though.
// Seeking line by line is easier (using "readline()") and can be 
// buffered but I don't know how expensive it is.

// Looks like it takes around 23ms to seek to an advanced line in 
// the wordlist:
/*
$ time head -n 400000 words.txt | tail -n 1
teadish

real	0m0.023s
user	0m0.020s
sys	0m0.000s
*/
// This is annoying to do repeatedly but I could cache the results
// that have been seen already. That structure needs a limit to 
// how much data it can hold though.
// Also, I'm aware the CLI and piping all of the lines until the 
// one I want should be slower than what I'll do in Rust.

// Using open instead of new, that's what they do 
// with the File struct to return a Result.
impl WordlistPseudoyimizer {

  pub fn open(filename: &str) -> Result<WordlistPseudoyimizer> {
    match File::open(filename) {
      Err(why) => Err(eyre!("Could not open file {} - {}", filename, why)),
      Ok(file) => {
        match line_count(&file) {
          Err(why) => Err(eyre!("Could not count lines in file - {}", why)),
          Ok(line_count) => Ok(
            WordlistPseudoyimizer {
              file,
              line_count,
              cache: Cache::new(CACHE_CAPACITY)
            }
          )
        }
      }
    }
  }

}

fn line_count<R>(handle: R) 
-> Result<usize>
where R: Read {
  let lines_c = count_lines(handle)
    .context("Counting lines in word list")?;
  if lines_c < 1 {
    Err(eyre!("Source file needs to have at least one line (I suggest more)"))
  } else {
    Ok(lines_c)
  }
}

// Improvised caching system, thought of using either a Vec
// or a LinkedList, because I wanted to move the last seen
// hashes to the front of the structure. Not sure that's 
// worth it as browsing a Vec is really quick anyway but 
// needing to move items around isn't something a Vec is 
// made for and I need it for the automatic entry 
// "expires when structure is full" mechanism I wanted.
// In short, I don't know if a linked list is more effective
// than shifting a whole bunch of elements in a Vec everytime.
// However, they say in the Rust docs that array-based data
// structures are often always faster because CPU cache and
// CPUS BE FAST.
type CacheEntry = (u64, String);

struct Cache {
  cache: VecDeque<CacheEntry>,
  capacity: usize
}

impl Cache {

  pub fn new(capacity: usize) -> Self {
    Self {
      capacity,
      cache: VecDeque::with_capacity(capacity)
    }
  }

  pub fn add(&mut self, entry: CacheEntry) {
    // If we're at capacity, pop an item:
    if self.cache.len() >= self.capacity {
      self.cache.pop_front();
    }
    self.cache.push_back(entry);
  }

  pub fn get(&self, hash: u64) -> Option<&CacheEntry> {
    // Iterate in reverse:
    for entry in self.cache.iter().rev() {
      if entry.0 == hash {
        return Some(entry);
      }
    }
    None
  }

}

#[cfg(test)]
mod tests {
  use super::*;

  // "words.txt", included in the repo, is also a test fixture.
  // Should probably document that somewhere.

  #[test]
  fn count_lines_in_wordlist() {
    /*match File::open("./resources/words.txt") {
      Err(why) => panic!("couldn't open wordlist - {}", why),
      Ok(file) => assert_eq!(line_count(file).unwrap(), 466462),
    };*/
    let sut = WordlistPseudoyimizer::open("./resources/words.txt").unwrap();
    assert_eq!(sut.line_count, 466462);
  }

  #[test]
  fn cache_miss_on_empty_cache() {
    let cache = Cache::new(CACHE_CAPACITY);
    let miss = cache.get(2389472);
    assert_eq!(miss, None);
  }

  #[test]
  fn cache_hit_and_miss() {
    let mut cache = Cache::new(CACHE_CAPACITY);
    cache.add((3, String::from("3")));
    cache.add((6, String::from("6")));
    let miss = cache.get(2389472);
    let hit = cache.get(3).unwrap();
    assert_eq!(miss, None);
    assert_eq!(3, hit.0);
    assert_eq!("3", hit.1);
  }

}

