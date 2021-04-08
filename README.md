# dkvz-blog-backend
Thought it was time to ditch Java and try something more efficient for a small server.

I still don't know how to Rust very well.

Database is probably missing until I decide to upload an empty one.

Using **eyre** instead of **failure** for errors and stuff.

# Endpoints
I think I never really documented them. Now is probably a good time.

## / - GET
Currently returns the text "nothing here" - I might be returning JSON from now on.

## /tags - GET
Gets the full list of tags in JSON format, ordered by name ASC.

Example with a single tag:
```json
[
  {
    "name": "Art & Beauté",
    "mainTag": 1,
    "id": 7
  },
]
```

## /article/{articleUrl} - GET
Gets the requested article in JSON format. Can use an article ID instead of the URL slug.

Throws a 404 if nothing is found.

Format differs slighly with shorts and full articles.

Full article:
```json
"date": "18/02/2021 17:40:21+0100",
"summary": "...",
"thumbImage": "stuff/img.png",
"author": "DkVZ",
"commentsCount": "0",
"id": "120",
"title": "Titre",
"articleURL": "truc_machin"
"content": "...",
"tags": [
  {
    "name": "Some tag",
    "id" : 2,
    "mainTag": true
  }
]
```

Short:
```json
"date": "18/02/2021 17:40:21+0100",
"summary": "...",
"thumbImage": "stuff/img.png",
"author": "DkVZ",
"commentsCount": "0",
"id": "120",
"title": "Titre",
"articleURL": null,
"content": "...",
"tags": []
```

We can remove articleURL completely for shorts. I think.

At the moment tags is always empty for shorts but I'm leaving it there just in case.

## /articles-starting-from/{start} - GET
Accepts a few extra query string params:
* max - Max amount of articles to get per request, defaults to 30.
* tags - Comma separated list of tag names (URL encoded by the client), defaults to empty string.
* order - expects the strings "asc" or "desc", defaults to "desc".

Returns a list of articles starting from the given article ID, which is used here as a very simple "offset".

**Completely ignores "short" and non-published articles**.

Responds with a list of posts with no "content" field when compared to the /article endpoint.

Responds with a 404 when requested starting point (referred to as "start") is past the article count for that request (takes tags into account). The endpoint actually queries for the article count internally before responding.

# /shorts-starting-from/{start} - GET
Same as the previous endpoint but fetches shorts. Result list also has the "content" field in this case.

## /comments - POST
Expects a URL encoded standard form submission format with fields:
* comment -> Limit length to 2000 chars
* author -> Trim + limit length to 70 chars - Refuse if length is 0 after trim with Bad Request
* article_id -> Supposed to be parsed from a string
* articleurl

We need either article_id or articleurl, Bad Request when both are absent.

Returns posted comment as JSON if it worked. Example output:
```json
"id": 299,
"author": "Paul",
"date": "dd/MM/yyyy HH:mm:ssZ",
"comment": "The actual comment"
```

## /last-comment - GET
Outputs the last comment or a 404 if there aren't any.

I need to add the article_id to the list of fields (probably as "articleId" since I use cameCase everywhere else).

## /import-articles - GET
Supposed to set a lock during the import so that another import cannot take place at the same time.

Sends that response when import is already in progress:
```json
"status": "error",
"message": "Import already in progress"
```
Altough having that exact format or not doesn't matter.

**This endpoint has to be publicly available but technically doesn't need CORS**.

When import works, we get a list such as the following:
```json
[
  {
    "status": "success",
    "message": "Article inserted",
    "id": 22
  }
]
```
Could technically be a mix of "success" and "error" as status.

Message explains if article or short was inserted (displays if it was an article or short), updated, deleted, or shows the relevant error message.

Specifically, we had "IO Error" and "JSON parsing error".

JSON format for the article files is similar to the regular article format except everything is optional and there are a few extra possible fields:
* "action": Will delete that article if set to 1 (and an "id" is present). Otherwise the action is decided by the presence of "id".
* "short": Pretty sure this is absent from the regular API responses. Defines if short or article, default is short I think.

I used to allow loose typing of the JSON and converting strings in various places, I don't anymore because it requires me creating custom deserializing routines and I couldn't bother for an enpoint only I'm using.

Here's an example import file:
```json
{
  "id":35,
  "articleURL":"some_url",
  "title":"Great title",
  "summary":"Salut\nEt oui",
  "content":"Contenu de l'article",
  "published":false,
  "thumbImage":"img.png",
  "userId": 2,
  "date": "2018-07-25T21:35:04.887Z",
  "tags": [
    {"id":21},
    {"id":44}
  ],
  "short": false
}
```

Published being absent used to mean it got set to false forcibly. This *may* change with the new API in that I may not update published if it's absent.

The endpoint slightly changed in that it's now only deleting the files that were successfully processed from the import directory (it used to delete everything that could be parsed AFAIK).

# /articles/search - POST
I'm using a weird rate limiter on that endpoint which basically blocks (with Forbidden HTTP error) ALL searches when a certain threshold is reached.

Expects a specific JSON body:
```json
include: [
  "search term 1",
  "search term 2"
]
```

Will respond with Bad Request if the include array is empty or null, which implies it going into a cleaning up function that replaces a few special chars as in the following Java Regex:
```
"[+*$%\\s]"
```
Which I think doesn't actually replace `+` or `*` at the moment. And we could allow `*` as it works in SQLite fulltext search queries.

When everything goes right, sends a list of "search results":
```json
"id": 34,
"title": "Some article title",
"articleURL": "some_url",
"snippet": " [...] Data from fulltext search "
```
Where "articleURL" is the article ID as string for shorts.

## /rss - GET
Only works for a set of allowed IP addresses or returns a forbidden exception.

Outputs the full RSS feed as XML, all published articles in descending order.

## /gimme-sitemap - GET
Returns the sitemap as "application/xml" MIME type. No CORS required.

Gets all the articles and shorts. Used to post all the articles first then all the shorts.

Query string parameters:
* articlesRoot - Defaults to "dkvz.eu/articles" - Absolute article URLs are created from it.

## /rebuild-indexes - GET
Only works for a set of allowed IP addresses or returns a forbidden exception.

Supposed to set a lock so that you can't run two of these at the same time.

Rebuilds the fulltext index completely.

## /render-article/{articleUrl} - GET
Renders a barebones version of the full article page in HTML for search engines. Doesn't need any CORS.

Will require setting up templating of some sort. I might put an example in resources later on - Previous backend was using a Thymeleaf template, maybe we can reuse it?

## Database
Some of the database workings were inspired by this example: https://github.com/actix/examples/tree/master/async_db

## Uselful links
* [Data access class from current backend](https://github.com/dkvz/DoradeBlogEngineSpring/blob/master/src/main/java/eu/dkvz/BlogAuthoring/model/BlogDataAccessSpring.java)

## IP Location
I'm using ip2location, more precisely the DB5.LITE from here https://lite.ip2location.com/ip2location-lite. I guess I'll be using the IPv4 BIN.

Rust library that looks promising: https://github.com/marirs/rust-ip2location.

They have the DB committed on Github but I thought I probably shouldn't.

## Logging
The crate [env_logger](https://docs.rs/env_logger/0.8.3/env_logger) integreates with Actix but I can also use it in my StatsService.

Got version 0.7 in my Actix notes.

We also need the "log" crate explicitely imported:
```
log = "0.4.0"
env_logger = "0.8.3"
```

You basically use it like so:
```rs
use log::{debug, error, log_enabled, info, Level};

env_logger::init();

debug!("this is a debug {}", "message");
error!("this is printed by default");

if log_enabled!(Level::Info) {
  let x = 3 * 4; // expensive computation
  info!("the answer was: {}", x);
}
```
Provided the `RUST_LOG` env variable is set.

For Actix they were manually setting it to "info" with the following line (before the call to init()):
```rs
std::env::set_var("RUST_LOG", "actix_web=info");
```
There's another interesting approach, combining .env, here: https://github.com/fairingrey/actix-realworld-example-app/blob/master/src/main.rs

The RUST_LOG value shown above won't show any log message from sources that aren't actix_web, which isn't ideal.

I use it like so, which means default log level for sources is info, and then specify a log level for actix_web (which incidentally is also info, but you get the point):
```
std::env::set_var("RUST_LOG", "info,actix_web=info");
```

## CORS
I think there's an example in the official "examples" repo, otherwise this middleware sounds promising: https://github.com/actix/examples/tree/master/web-cors

## Custom errors
I wanted to have some text with the default 404 responses that are sent when no route handler matches the request (I think there's also another error when path or query matching failed) and at first I tried the ErrorHandlers middleware with the following wrap call:

```
.wrap(
  ErrorHandlers::new()
      .handler(http::StatusCode::NOT_FOUND, render_404)
)
```
And this handler function:
```rs
fn render_404<B>(mut res: dev::ServiceResponse<B>) -> actix_web::Result<ErrorHandlerResponse<B>> {
  /* OK so that works:
  let new_res: dev::ServiceResponse<B> = res.map_body(|_, _| {
      body::ResponseBody::Other(body::Body::Message(Box::new("404 LOL")))
  });
  Ok(ErrorHandlerResponse::Response(new_res))
  */
  // But it doesn't add the content type header, which 
  // was present before with the default error implementation.
  // Alright so this was way more complicated than I thought 
  // but there aren't many examples of using this middleware
  // online.
  let mut resp = res.map_body::<_, B>(|_, _| {
    body::ResponseBody::Other(body::Body::Message(Box::new("404 LOL")))
  });
  resp.headers_mut()
    .insert(
      http::header::CONTENT_TYPE,
      http::HeaderValue::from_static("text/plain; charset=utf-8"),
    );
  Ok(ErrorHandlerResponse::Response(resp))
}
```

And it does work but it intercepts every single 404 responses and replaces them, including the one I already create myself as custom errors.

I would need to inject some kind of way to recognize my own errors from the default Actix ones in the response, not impossible but painful.

Another lead would be to use a ''default_service'' which calls a handler when no route match a request, which is kinda like where you'd expect to send a 404.

It's explained here: https://docs.rs/actix-web/3.3.2/actix_web/struct.App.html#method.default_service

### Customizing JSON errors
I think this only applies to requests with JSON bodies, but there's some app_data() that can be injected and is explained in the JSON section of this page: https://actix.rs/docs/extractors

### Customizing path and query extractor errors
It looks like it's super complicated to do. I could use the first method described on top of the section with the middleware... Or implement the whole extraction logic inside of the function. But I couldn't get that to work for the query params.

I'll just leave these errors as is, looks like Actix isn't as flushed out as I thought.

After some more digging, I found this: https://docs.rs/actix-web/3.3.2/actix_web/web/struct.PathConfig.html

And the equivalent for query params: https://docs.rs/actix-web/3.3.2/actix_web/web/struct.QueryConfig.html

We need both.

# TODO
- [x] I need a generic function for "count" queries.
- [x] Log a message when server is started -> Actix already does that.
- [x] IP+port should be configurable from the .env with some kind of default value maybe?
- [x] Make some generic way to convert to DTOs in request handlers, I probably need a trait -> From seems to work on vectors so From is all I need.
- [x] A custom 404 message for invalid URLs would be nice
- [x] Do I also need a custom BadRequest or whatever is sent when you provide invalid path params?
- [x] I'm still missing custom errors for request query params.
- [ ] I'm allowing importing articles with article_url being null, as if they were short. Does that cause weird fatal errors?
- [ ] Can we use web::FormConfig to limit the size of form POST requests?
- [ ] It would make sense for the rate limiter to be a "guard".
- [ ] Try to see a database error on purpose, not sure if it even works.
- [ ] DB errors should be their own custom error type so that I could very easily have a From trait for app::error::Error to convert them into that.
- [ ] Try selecting only the features I need from dependencies and see if that reduces the binary size - I don't think I need the whole serde crate.
- [ ] Try reorganizing the giant closure that is in StatsService::open. We could open the iploc and pseudonymizer inside of a function given to spawn() and have the loop happen after that.
- [ ] What happens if you request a negative article ID?
- [ ] I need da CORS. 
- [x] Fields like thumb_image and article_url can be NULL; Does Option automatically work in the entity?
- [x] Make the stats thread message queue size configurable! Could also probably set it to be larger by default.
- [x] None of the plain text and "default error messages" (like when an endpoint fails parsing a path variable) specify encoding, so browsers are using US-ASCII and that's a problem. I'm missing "content-type: text/plain; charset=utf-8".
- [x] Does encoding actually work with the JSON endpoints?
- [x] Should use a Logger instead of println! inside of StatsService, I should be able to use the log crate.
- [x] The Query struct doesn't need to get vectors, we could give slices of arrays instead.
- [ ] Do shorts get inserted with content NULL or empty string?
- [x] Forgot to replace some special chars before inserting the fulltext data ("<" and ">") - Used to to this with JSoup.
- [ ] Check that special chars and HTML is removed from the fulltext inserts and updates.
- [ ] full_article_mapper should probably take a Connection instead of a Pool.
- [ ] To re-test: article insertion, article update, rebuilding fulltext index entirely.
- [ ] Dates could be options in entities, I could just unwrap_or to a function that gets the current date in insert functions.
- [ ] Test all the comment DB functions.
- [ ] I need a specific "entity" for search results. Or not? The weird empty thumb image and empty tags vector are making me feel bad.
- [x] Create a limited length fixture instead of the full wordlist.
- [ ] I'm not sure cloning the connection pool for almost every request is the way to go in db/mod.rs. Maybe it's how the "pool" gets used the most efficienctly though.
- [ ] Similar remark with cloning the SyncSender in stats/mod.rs, search for "TODO".
- [ ] I get the author name in full_article_mapper for every single article (old API does that too), I should add it to the actual query, possibly with a LEFT JOIN. I'm not sure that would even be faster though, especially with SQLite. Search function already includes fetching username in the query.
- [ ] In article_import.rs, the whole loop in "files" that saves to DB should be a Tokio blocking task.