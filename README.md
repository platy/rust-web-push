Rust Web Push
=============

[![Travis Build Status](https://travis-ci.org/pimeys/rust-web-push.svg?branch=master)](https://travis-ci.org/pimeys/rust-web-push)
[![crates.io](http://meritbadge.herokuapp.com/web_push)](https://crates.io/crates/web_push)

Web push notification sender.

## Version 0.8

With version 0.8 this project has changed maintainer (thanks @pimeys), the chrome-specific handling has been removed (as the web push api is standard across browsers now), and this doesn't require tokio anymore.

If you were using 0.7 the migration would be, in Cargo.toml we now use tokio version 1.1, and:

```diff
- web-push = "0.7"
+ web-push = { version="0.8", features = ["http-hyper"], default-features = false }
```

And in the code use the tokio version of the client:
```diff
- web_push::WebPushClient::new()
+ web_push::TokioWebPushClient::new()
```
and don't set the firebase key:
```diff
- builder.set_gcm_key(gcm_key);
```

## Feature flags

A feature flag can be used to choose which http client to use, either:

- `http-hyper` - using the async `hyper` client which requires a `tokio` runtime and enables `web_push::TokioWebPushClient`
- `http-ureq` - using the blocking `ureq` client which will block and enables `web_push::BlockingWebPushClient`

Both are enabled by default.

Documentation
-------------

* [Released](https://docs.rs/web-push/)
* [Master](https://pimeys.github.io/rust-web-push/master/index.html)

To send a web push from command line, first subscribe to receive push
notifications with your browser and store the subscription info into a json
file. It should have the following content:

``` json
{
  "endpoint": "https://updates.push.services.mozilla.com/wpush/v1/TOKEN",
  "keys": {
    "auth": "####secret####",
    "p256dh": "####public_key####"
  }
}
```

Google has [good instructions](https://developers.google.com/web/updates/2015/03/push-notifications-on-the-open-web) for
building a frontend to receive notifications.

Store the subscription info to `examples/test.json` and send a notification with
`cargo run --example simple_send -- -f examples/test.json -p "It works!"`.

Examples
--------

To see it used in a real project, take a look to the [XORC
Notifications](https://github.com/xray-tech/xorc-notifications), which is a
full-fledged consumer for sending push notifications.

VAPID
-----

VAPID authentication prevents unknown sources sending notifications to the
client and allows sending notifications without any third party account.

The private key to be used by the server can be generated with OpenSSL:

```
openssl ecparam -genkey -name prime256v1 -out private_key.pem
```

To derive a public key from the just-generated private key, to be used in the
JavaScript client:

```
openssl ec -in private_key.pem -pubout -outform DER|tail -c 65|base64|tr '/+' '_-'|tr -d '\n'
```

The signature is created with `VapidSignatureBuilder`. It automatically adds the
required claims `aud` and `exp`. Adding these claims to the builder manually
will override the default values.

Overview
--------

Currently implements
[HTTP-ECE Draft-3](https://datatracker.ietf.org/doc/draft-ietf-httpbis-encryption-encoding/03/?include_text=1)
content encryption for notification payloads. The client requires
[Tokio](https://tokio.rs) for asynchronious requests if using the `async-hyper` feature. The modular design allows
an easy extension for the upcoming aes128gcm when the browsers are getting
support for it.

Tested with Google's and Mozilla's push notification services.

Debugging
--------
If you get an error or the push notification doesn't work you can try to debug using the following instructions:

Add the following to your Cargo.toml:
```cargo
log = "0.4"
pretty_env_logger = "0.3"
```

Add the following to your main.rs:
```rust
extern crate pretty_env_logger;
// ...
fn main() {
  pretty_env_logger::init();
  // ...
}
```

Or use any other logging library compatible with https://docs.rs/log/

Then run your program with the following environment variables:
```bash
RUST_LOG="web_push::client=trace" cargo run
```

This should print some more information about the requests to the push service which may aid you or somebody else in finding the error.
