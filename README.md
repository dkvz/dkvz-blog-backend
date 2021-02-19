# dkvz-blog-backend
Thought it was time to ditch Java and try something more efficient for a small server.

I still don't know how to Rust very well.

Database is probably missing until I decide to upload an empty one.

Using **eyre** instead of **failure** for errors and stuff.

## Database
Some of the database workings were inspired by this example: https://github.com/actix/examples/tree/master/async_db

## Uselful links
* [Data access class from current backend](https://github.com/dkvz/DoradeBlogEngineSpring/blob/master/src/main/java/eu/dkvz/BlogAuthoring/model/BlogDataAccessSpring.java)

## IP Location
I'm using ip2location, more precisely the DB5.LITE from here https://lite.ip2location.com/ip2location-lite. I guess I'll be using the IPv4 BIN.

Rust library that looks promising: https://github.com/marirs/rust-ip2location.

They have the DB committed on Github but I thought I probably shouldn't. 

# TODO
- [x] I need a generic function for "count" queries.
- [ ] Try selecting only the features I need from dependencies and see if that reduces the binary size - I don't think I need the whole serde crate.
- [x] The Query struct doesn't need to get vectors, we could give slices of arrays instead.
- [ ] Do shorts get inserted with content NULL or empty string?
- [x] Forgot to replace some special chars before inserting the fulltext data ("<" and ">") - Used to to this with JSoup.
- [ ] Check that special chars and HTML is removed from the fulltext inserts and updates.
- [ ] full_article_mapper should probably take a Connection instead of a Pool.
- [ ] I'm still debating whether SQL errors should cause full program crash when it'll be running as an Actix server.
- [ ] To re-test: article insertion, article update, rebuilding fulltext index entirely.
- [ ] Dates could be options in entities, I could just unwrap_or to a function that gets the current date in insert functions.
- [ ] Test all the comment DB functions.
- [ ] I need a specific "entity" for search results. Or not? The weird empty thumb image and empty tags vector are making me feel bad.
- [x] Create a limited length fixture instead of the full wordlist.