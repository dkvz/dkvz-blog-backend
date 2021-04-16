use serde::{Deserialize, Serialize};
use derive_more::Display;
use super::helpers;
use crate::db::entities::*;
use crate::utils::{
  self, 
  time_utils, 
  serde_utils,
  text_utils
};
use crate::config::SiteInfo;

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
  #[serde(skip_serializing_if = "Option::is_none")]
  pub article_id: Option<i32>,
  pub author: String,
  pub comment: String,
  pub date: String
}

impl From<Comment> for CommentDto {
  fn from(comment: Comment) -> Self {
    Self {
      id: comment.id,
      article_id: Some(comment.article_id),
      author: comment.author,
      comment: comment.comment,
      date: time_utils::timestamp_to_date_string(comment.date)
    }
  }
}

// At some point I decided it would be nice to save
// some bytes when sending the list of comments for
// an article and so I removed article_id. I don't
// know why I bothered but that's the story of my 
// life.
impl CommentDto {
  pub fn remove_article_id(mut self) -> Self {
    self.article_id = None;
    self
  }
}

// I actually have to be strict with what
// types I allow in the JSON or I'd have
// to create custom deserializing functions
// as shown here:
// https://stackoverflow.com/questions/37870428/convert-two-types-into-a-single-type-with-serde
#[derive(Debug, Serialize, Deserialize, Clone)]
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
  // thumb_image is a special case for which we allow
  // nullifying the field in DB if the update JSON
  // had the field set to null. We use a double Option
  // and a special deserializer.
  #[serde(
    default, 
    deserialize_with = "serde_utils::deserialize_null_value"
  )]
  pub thumb_image: Option<Option<String>>,
  // The date is ignored by the import service.
  // I didn't know.
  //pub date: Option<String>,
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

// Empty strings and useless comment count are required
// because these objects are  #[serde(deserialize_with = "serde_utils::empty_string_is_none")] also used for displaying
// the actual representation of the article.
impl From<ImportedArticleDto> for Article {
  fn from(dto: ImportedArticleDto) -> Self {
    // We completely ignore the ID if any.
    // My authoring software is writing empty strings that
    // should become NULL in database, hence this hack:
    let article_url = serde_utils::empty_string_to_none(
      if dto.article_url.is_some()
        { dto.article_url } else { dto.article_url_bis }
    );
    Self {
      id: -1,
      title: dto.title.unwrap_or(String::new()),
      article_url,
      //thumb_image: serde_utils::empty_string_to_none(dto.thumb_image),
      thumb_image: dto.thumb_image.and_then(
        serde_utils::empty_string_to_none
      ),
      date: time_utils::current_timestamp(),
      user_id: dto.user_id.unwrap_or(1),
      summary: dto.summary.unwrap_or(String::new()),
      content: dto.content,
      published: utils::option_bool_to_i32(dto.published),
      short: utils::option_bool_to_i32(dto.short),
      tags: dto.tags.map(
        |v| v.into_iter().map(|a| Tag::from(a)).collect()
      ).unwrap_or(Vec::new()),
      // The field is ignored, should probably be an
      // option but I couldn't be bother to refactor.
      author: String::new(),
      comments_count: 0
    }
  }
}

impl From<ImportedArticleDto> for ArticleUpdate {
  fn from(dto: ImportedArticleDto) -> Self {
    Self {
      // Kinda stupid but I don't want to crash the 
      // program in here:
      id: dto.id.unwrap_or(0),
      title: dto.title,
      article_url: dto.article_url,
      thumb_image: dto.thumb_image,
      user_id: dto.user_id,
      summary: dto.summary,
      content: dto.content,
      published: dto.published.map(
        |p| match p {
          true => 1,
          false => 0
        }
      ),
      tags: dto.tags.map(
        |v| v.into_iter().map(|a| Tag::from(a)).collect()
      )
    }
  }
}

// I need this for the tag deserialization
// to work with the article import process:
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportedArticleTagDto {
  pub id: i32,
  pub name: Option<String>
}

// There's a lot of useless empty strings in these
// entity conversions but that's the easiest way
// I found to work with the same struct for input
// and output.
impl From<ImportedArticleTagDto> for Tag {
  fn from(dto: ImportedArticleTagDto) -> Self {
    Self {
      id: dto.id,
      name: dto.name.unwrap_or(String::new()),
      // lol that main_tag thing, I don't 
      // even remember what it was supposed
      // to be.
      main_tag: 1
    }
  }
}

// Format I'm using for the search requests. The
// include thingy is kinda useless but it's historical.
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchBody {
  pub include: Vec<String>
}

// The object that respresents search results:
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
  pub snippet: String,
  pub id: i32,
  pub title: String,
  #[serde(rename = "articleURL")]
  pub article_url: String,
  pub date: String
}

impl From<Article> for SearchResult {
  fn from(article: Article) -> Self {
    // The "snippet" is mapped into the summary
    // by the DB function.
    // Use the ID as article URL if it's a short:
    let article_url = match article.short {
      1 => article.id.to_string(),
      _ => article.article_url.unwrap_or(article.id.to_string())
    };
    Self {
      snippet: article.summary,
      id: article.id,
      title: article.title,
      article_url,
      date: time_utils::timestamp_to_date_string(article.date)
    }
  }
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

// Following stuct is used by the template
// engine to generate the RSS feed file.
// Using &str in there just because I 
// wanted to see if it'd work.
#[derive(Serialize)]
pub struct RssFeed<'a> {
  pub title: &'a str,
  pub root: &'a str,
  pub articles_root: &'a str,
  pub shorts_root: &'a str,
  pub description: &'a str,
  pub build_date: String,
  pub rss_full_url: &'a str,
  pub items: Vec<RssFeedEntry>,
  max_rss_length: usize
}

impl<'a> RssFeed<'a> {
  pub fn new(site_info: &'a SiteInfo, max_rss_length: usize) -> Self {
    Self {
      title: &site_info.title,
      root: &site_info.root,
      articles_root: &site_info.articles_root,
      shorts_root: &site_info.shorts_root,
      description: &site_info.description,
      build_date: time_utils::current_datetime_rfc2822(),
      rss_full_url: &site_info.rss_full_url,
      items: Vec::new(),
      max_rss_length
    }
  }

  // We want to move the Article in there, it shouldn't
  // be used afterwards. Hoping to save some memory this
  // way but I have no idea if it actually does.
  pub fn add_item(&mut self, article: Article) {
    // Create the link by checking if it's a short or not:
    let link = match article.short {
      1 => helpers::generate_article_url(
        self.root, 
        self.shorts_root, 
        article.id.to_string()
      ),
      _ => helpers::generate_article_url(
        self.root, 
        self.articles_root,
        article.article_url.unwrap_or(article.id.to_string())
      )
    };
    let media = article.thumb_image
      .map(|url| {
        // Check if we have to add a "/" or not:
        match url.find('/') {
          Some(0) => format!("{}{}", self.root, url),
          _ => if url.find("://").is_none() {
            format!("{}/{}", self.root, url)
          } else {
            // URL appears to not be relative.
            url
          }
        }
      });
    // Check if description is smaller than the max allowed size
    // for descriptions in the RSS feed:
    let mut description = article.content.unwrap_or(article.summary);
    // We could use truncate but it can panic if the truncate point
    // is in between two or more bytes of the same char.
    // This is due to Rust not using chars but bytes at the core.
    // len() actually reports the byte size too, but I don't care, 
    // I consider the resize once a certain byte size is reached:
    if description.len() > self.max_rss_length {
      description = description
        .chars()
        .take(self.max_rss_length)
        .collect();
      // Push the extra text with the full article link:
      description.push_str(&format!(
        "...<p><b><a href=\"{}\">Suite disponible sur le site</a></b></p>",
        link
      ));
    }
    // Replace all the relative URLs with absolute ones.
    // We also used to escape HTML entities here, but not
    // only handlebars can do it, but also I'm putting the
    // content in a CDATA block and so it should absolutely
    // not be escaped in any way.
    let description = text_utils::relative_links_to_absolute(
      &description, 
      self.root
    );

    self.items.push(
      RssFeedEntry {
        title: article.title,
        link,
        date: time_utils::current_datetime_rfc2822(),
        media,
        description: description.to_string()
      }
    );
  }
}

#[derive(Serialize)]
pub struct RssFeedEntry {
  pub title: String,
  pub link: String,
  pub date: String,
  pub media: Option<String>,
  pub description: String
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

  #[test]
  fn empty_string_is_none_for_article_thumb_image() {
    let sut = ImportedArticleDto {
      action: None,
      article_url: None,
      article_url_bis: None,
      content: None,
      id: None,
      published: None,
      short: None,
      summary: None,
      tags: None,
      title: None,
      user_id: None,
      thumb_image: Some(Some("".to_string()))
    };
    let article: Article = sut.into();
    assert_eq!(article.thumb_image, None);
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
