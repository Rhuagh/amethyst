extern crate amethyst_assets;
extern crate amethyst_core;
#[macro_use]
extern crate serde;
extern crate shred;
#[macro_use]
extern crate shred_derive;

#[cfg(feature = "profiler")]
extern crate thread_profiler;

pub mod circular_buffer;
pub mod fps_counter;
pub mod tag;
