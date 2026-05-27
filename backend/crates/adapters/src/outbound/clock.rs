//! System clock adapter — placeholder.
//!
//! No `Clock` port is exposed by `domain` / `application` yet. This file
//! reserves the module path for the upcoming monitoring use cases that need
//! "now" semantics (issues #8+). It will host a `SystemClock` struct
//! implementing the future `domain::Clock` port.
