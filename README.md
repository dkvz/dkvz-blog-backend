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

# TODO
- [x] I need a generic function for "count" queries.
- [x] Log a message when server is started -> Actix already does that.
- [x] IP+port should be configurable from the .env with some kind of default value maybe?
- [ ] Try selecting only the features I need from dependencies and see if that reduces the binary size - I don't think I need the whole serde crate.
- [ ] Try reorganizing the giant closure that is in StatsService::open. We could open the iploc and pseudonymizer inside of a function given to spawn() and have the loop happen after that.
- [x] Should use a Logger instead of println! inside of StatsService, I should be able to use the log crate.
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
- [ ] I'm not sure cloning the connection pool for almost every request is the way to go in db/mod.rs.
- [ ] Similar remark with cloning the SyncSender in stats/mod.rs, search for "TODO".