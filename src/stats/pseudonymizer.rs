use color_eyre::Result;
use eyre::{eyre, WrapErr};
use linecount::count_lines;
use sha1::{Digest, Sha1};
use std::collections::VecDeque;
use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

// Capacity of the queue I use for caching
const CACHE_CAPACITY: usize = 50;
// Max value for indexing words
const MAX: u64 = u64::MAX;

pub struct WordlistPseudoyimizer {
  filename: String,
  line_count: u64,
  cache: Cache,
  increment: u64,
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
      Ok(file) => match line_count(&file) {
        Err(why) => Err(eyre!("Could not count lines in file - {}", why)),
        Ok(line_count) => Ok(WordlistPseudoyimizer {
          filename: String::from(filename),
          line_count,
          cache: Cache::new(CACHE_CAPACITY),
          increment: (MAX / line_count) + 1,
        }),
      },
    }
  }

  // Starts at line 0 to line_count - 1!
  fn find_value_at_line(&self, line: u64) -> Result<String> {
    // Buffer through the file from the start as explained here:
    // https://doc.rust-lang.org/rust-by-example/std_misc/file/read_lines.html
    // I have to reopen a File handle everytime because reusing one
    // doesn't work. Or I don't know how to make it work.
    let file = File::open(&self.filename)?;
    let reader = BufReader::new(file).lines();
    // If requested line number is higher than total count of
    // lines we just loop over and start again from 0.
    // Also makes asing for "line_count" result in line 0, which
    // is nice.
    let line_n = line % self.line_count;
    let mut i: u64 = 0;
    for line in reader {
      if i == line_n {
        return line.context("Could not read line in word list");
      }
      i += 1;
    }
    Err(eyre!(
      "Went to the very end of the wordlist for line {} - This shouldn't happen",
      line_n
    ))
  }

  pub fn pseudonymize(&mut self, value: &str) -> Result<String> {
    // Hash the value:
    let hash = bytes_to_u64(hash_to_8_bytes(value));
    // Check if we have it in cache, add it otherwise.
    match self.cache.get(hash) {
      Some(entry) => Ok(entry.value.clone()),
      None => {
        let pseudo = self.find_value_at_line(hash / &self.increment)?;
        self.cache.add(CacheEntry::new(hash, pseudo.clone()));
        Ok(pseudo)
      }
    }
  }
}

fn line_count<R>(handle: R) -> Result<u64>
where
  R: Read,
{
  let lines_c = count_lines(handle).context("Counting lines in word list")?;
  if lines_c < 1 {
    Err(eyre!(
      "Source file needs to have at least one line (I suggest more)"
    ))
  } else {
    Ok(u64::try_from(lines_c)?)
  }
}

fn hash_to_8_bytes(value: &str) -> [u8; 8] {
  let mut hasher = Sha1::new();
  hasher.update(value.as_bytes());
  // acquire hash digest in the form of GenericArray,
  // which in this case is equivalent to [u8; 20]
  let result = hasher.finalize();
  // Take a slice of byte o to 7, should never error:
  result[0..8].try_into().unwrap()
}

fn bytes_to_u64(hash: [u8; 8]) -> u64 {
  // Convert to u64 using the current platform preferred
  // endian (little endian or big):
  u64::from_ne_bytes(hash)
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
#[derive(PartialEq, Debug, Clone)]
struct CacheEntry {
  key: u64,
  value: String,
}

impl CacheEntry {
  pub fn new(key: u64, value: String) -> Self {
    Self { key, value }
  }
}

struct Cache {
  cache: VecDeque<CacheEntry>,
  capacity: usize,
}

impl Cache {
  pub fn new(capacity: usize) -> Self {
    Self {
      capacity,
      cache: VecDeque::with_capacity(capacity),
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
      if entry.key == hash {
        return Some(&entry);
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
  fn find_line_in_fixture() {
    let sut = WordlistPseudoyimizer::open("./resources/fixtures/fixed_wordlist.txt").unwrap();
    assert_eq!("Line 1", sut.find_value_at_line(0).unwrap());
  }

  #[test]
  fn pseudonymize_string_1() {
    let mut sut = WordlistPseudoyimizer::open("./resources/fixtures/fixed_wordlist.txt").unwrap();
    assert_eq!("Line 11", sut.pseudonymize("test").unwrap());
    // Test the cache, I guess:
    assert_eq!(1, sut.cache.cache.len());
    assert_eq!("Line 11", sut.pseudonymize("test").unwrap());
  }

  #[test]
  fn pseudonymize_string_2() {
    let mut sut = WordlistPseudoyimizer::open("./resources/fixtures/fixed_wordlist.txt").unwrap();
    assert_eq!(
      "Line 3",
      sut
        .pseudonymize("This is a very long string right there")
        .unwrap()
    );
  }

  #[test]
  fn can_clone_cache_entry() {
    // Might look like a stupid test but I needed to know.
    // In the end I'm not even cloning cache entries. Oh well.
    let sut = CacheEntry::new(22, String::from("value"));
    let mut clone = sut.clone();
    assert_eq!(sut, clone);
    clone.value = "changed".to_string();
    assert_ne!(sut.value, clone.value);
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
    cache.add(CacheEntry::new(3, String::from("3")));
    cache.add(CacheEntry::new(6, String::from("6")));
    let miss = cache.get(2389472);
    let hit = cache.get(3).unwrap();
    assert_eq!(miss, None);
    assert_eq!(3, hit.key);
    assert_eq!("3", hit.value);
  }

  #[test]
  fn sha1_extract_first_8_bytes() {
    // Take a slice of byte o to 7:
    let extract: [u8; 8] = hash_to_8_bytes("hello world");
    assert_eq!(extract.len(), 8);
    assert_eq!(extract, [42, 174, 108, 53, 201, 79, 207, 180]);
  }

  #[test]
  fn hash_to_u64() {
    let bytes = hash_to_8_bytes("hello world");
    let value: u64 = bytes_to_u64(bytes);
    assert_eq!(value, 13028719972609469994);
  }
}
