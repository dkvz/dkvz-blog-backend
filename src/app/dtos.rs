use crate::db::entities::*;

// I'm going to use the From trait to convert
// entites to DTOs and test that.
// I could make sure it works both ways but I
// really only need it entity -> DTO.

// The TagDto is actually exactly Tag. Can I 
// just re-export the entity?
pub use crate::db::entities::Tag as TagDto;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tag_to_dto() {
    let sut = Tag {
      id: 12,
      name: "Some Tag".to_string(),
      main_tag: 1
    };
    // into() moves ownership. I think.
    let dto: TagDto = sut.into();
    assert_eq!(12, dto.id);
  }

  #[test]
  fn vec_tag_to_vec_dto() {
    let t1 = Tag {
      id: 12,
      name: "Some Tag 1".to_string(),
      main_tag: 1
    };
    let t2 = Tag {
      id: 27,
      name: "Some Tag 2".to_string(),
      main_tag: 1
    };
    let sut: Vec<Tag> = vec![t1, t2];
    let converted: Vec<TagDto> = sut.into();
    assert_eq!(27, converted[1].id);
  }

}
