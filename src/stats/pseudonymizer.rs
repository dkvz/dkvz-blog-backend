use std::fs::File;
use std::io::prelude::*;
use linecount::count_lines;
use color_eyre::Result;
use eyre::{WrapErr, eyre};

pub struct WordlistPseudoyimizer {
  file: File,
  line_count: usize
}

// Using open instead of new, that's what they do 
// with the File struct to return a Result.
impl WordlistPseudoyimizer {

  pub fn open(filename: &str) -> Result<WordlistPseudoyimizer> {
    match File::open(filename) {
      Err(why) => Err(eyre!("Could not open file {} - {}", filename, why)),
      Ok(file) => {
        match line_count(&file) {
          Err(why) => Err(eyre!("Count not count lines in file - {}", why)),
          Ok(line_count) => Ok(
            WordlistPseudoyimizer {
              file,
              line_count
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
  count_lines(handle)
    .context("Counting lines in word list")
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
}

