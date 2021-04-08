use serde::{Deserialize, Serialize};
use derive_more::Display;
use crate::db::entities::*;
use crate::utils::{self, time_utils};

// I'm going to use the From trait to convert
// entites to DTOs and test that.
// I could make sure it works both ways but I
// really only need it entity -> DTO.

// The TagDto is actually exactly Tag. Can I 
// just re-export the entity?
pub use crate::db::entities::Tag as TagDto;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleDto {
  pub id: i32,
  pub date: String,
  pub summary: String,
  pub thumb_image: Option<String>,
  pub author: String,
  pub comments_count: i64,
  pub title: String,
  #[serde(rename = "articleURL")]
  pub article_url: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub content: Option<String>,
  pub tags: Vec<TagDto>
}

impl From<Article> for ArticleDto {
  fn from(article: Article) -> Self {
    Self {
      id: article.id,
      date: time_utils::timestamp_to_date_string(article.date),
      summary: article.summary,
      thumb_image: article.thumb_image,
      author: article.author,
      comments_count: article.comments_count,
      title: article.title,
      article_url: article.article_url,
      content: article.content,
      tags: article.tags
    }
  }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentDto {
  pub id: i32,
  pub article_id: i32,
  pub author: String,
  pub comment: String,
  pub date: String
}

impl From<Comment> for CommentDto {
  fn from(comment: Comment) -> Self {
    Self {
      id: comment.id,
      article_id: comment.article_id,
      author: comment.author,
      comment: comment.comment,
      date: time_utils::timestamp_to_date_string(comment.date)
    }
  }
}

// I actually have to be strict with what
// types I allow in the JSON or I'd have
// to create custom deserializing functions
// as shown here:
// https://stackoverflow.com/questions/37870428/convert-two-types-into-a-single-type-with-serde
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedArticleDto {
  pub id: Option<i32>,
  pub title: Option<String>,
  #[serde(rename = "articleURL")]
  pub article_url: Option<String>,
  // I historically allow two different 
  // key names for article_url:
  #[serde(rename = "articleUrl")]
  pub article_url_bis: Option<String>,
  pub thumb_image: Option<String>,
  // The date is a string in update requests:
  pub date: Option<String>,
  pub user_id: Option<i32>,
  pub summary: Option<String>,
  pub content: Option<String>,
  pub published: Option<bool>,
  pub tags: Option<Vec<ImportedArticleTagDto>>,
  pub short: Option<bool>,
  // Extra field to allow deletion when set 
  // to "1" or "delete":
  pub action: Option<u32>
}

impl From<ImportedArticleDto> for Article {
  fn from(dto: ImportedArticleDto) -> Self {
    // We completely ignore the ID if any.
    // Parse the weird date string:

    Self {
      id: -1,
      title: dto.title.unwrap_or(String::new()),
      article_url: dto.article_url,
      thumb_image: dto.thumb_image,
      date: 0,
      user_id: dto.user_id.unwrap_or(1),
      summary: dto.summary.unwrap_or(String::new()),
      content: dto.content,
      published: utils::option_bool_to_i32(dto.published),
      short: utils::option_bool_to_i32(dto.short),
      //tags: dto.tags.unwrap_or(Vec::new()),
      tags: Vec::new(),
      // The field is ignored, should probably be an
      // option but I couldn't be bother to refactor.
      author: String::new(),
      comments_count: 0
    }
  }
}

// I need this for the tag deserialization
// to work with the article import process:
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedArticleTagDto {
  pub id: i32,
  pub name: Option<String>,
  pub main_tag: Option<i32>
}

// I use this in some responses. Should probably use it
// for all of them but uh... Yeah.
#[derive(Debug, Deserialize, Serialize)]
pub struct JsonStatus {
  pub status: String,
  pub message: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<i32>
}

#[derive(Debug, Display)]
pub enum JsonStatusType {
  #[display(fmt = "success")]
  Success,
  #[display(fmt = "error")]
  Error
}

impl JsonStatus {
  pub fn new(status: JsonStatusType, message: &str) -> Self {
    Self {
      status: status.to_string(),
      message: String::from(message),
      id: None
    }
  }

  pub fn new_with_id(
    status: JsonStatusType, 
    message: &str, 
    id: i32
  ) -> Self {
    Self {
      status: status.to_string(),
      message: String::from(message),
      id: Some(id)
    }
  }
}

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

  /*
  let article = ArticleDto {
      article_url: Some("some_url".to_string()),
      id: 12,
      author: "Franck".to_string(),
      comments_count: 0,
      title: "Some title".to_string(),
      content: "Some content".to_string(),
      date: "01/02/2021".to_string(),
      summary: "Some summary".to_string(),
      thumb_image: "some_image.png".to_string(),
      tags: Vec::new()
    };
  */

}
