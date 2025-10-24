#![allow(non_snake_case)]

pub mod server;
pub mod stream;
pub mod floop; // funny way of getting around 'loop' being a keyword

mod sys;
mod thread_flag;
mod userdata;
