# dkvz-blog-backend
Thought it was time to ditch Java and try something more efficient for a small server.

I still don't know how to Rust very well.

Database is probably missing until I decide to upload an empty one.

Using **eyre** instead of **failure** for errors and stuff.

## Database
Some of the database workings were inspired by this example: https://github.com/actix/examples/tree/master/async_db

## Uselful links
* [Data access class from current backend](https://github.com/dkvz/DoradeBlogEngineSpring/blob/master/src/main/java/eu/dkvz/BlogAuthoring/model/BlogDataAccessSpring.java)

# TODO
- [ ] I need a generic function for "count" queries.
- [ ] Try selecting only the features I need from dependencies and see if that reduces the binary size - I don't think I need the whole serde crate.
- [ ] Test coverage is lacking in queries.rs.