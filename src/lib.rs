// src/lib.rs
#![deny(clippy::all)]

use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi::{JsFunction, Result};
use napi_derive::napi;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::thread::spawn;

mod get_browser_url;
mod listen_mouse_event;
mod window_process;

#[napi(ts_args_type = "callback: (event: string) => void")]
pub fn start_listener(callback: JsFunction) -> Result<()> {
  let jsfn: ThreadsafeFunction<String, ErrorStrategy::Fatal> =
    callback.create_threadsafe_function(0, |ctx| Ok(vec![ctx.value]))?;

  let jsfn = Arc::new(jsfn);

  spawn({
    let jsfn = Arc::clone(&jsfn);
    move || {
      let callback = Arc::new(Mutex::new(move |event: listen_mouse_event::MouseEvent| {
        let json_event = json!(event);
        jsfn.call(
          json_event.to_string(),
          ThreadsafeFunctionCallMode::NonBlocking,
        );
        ()
      }));
      listen_mouse_event::start(callback);
    }
  });

  Ok(())
}
