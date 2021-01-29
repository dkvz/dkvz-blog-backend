use std::fs::File;
use std::io::prelude::*;
use linecount::count_lines;
use color_eyre::Result;
use eyre::WrapErr;

pub struct WordlistPseudoyimizer {
  filename: String,
  line_count: usize
}

// Using open instead of new, that's what they do 
// with the File struct to return a Result.


pub fn line_count<R>(handle: R) 
-> Result<usize>
where R: Read {
  count_lines(handle)
    .context("Counting lines in word list")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn count_lines_in_wordlist() {
    match File::open("./resources/words.txt") {
      Err(why) => panic!("couldn't open wordlist - {}", why),
      Ok(file) => assert_eq!(line_count(file).unwrap(), 466462),
    };
  }
}

