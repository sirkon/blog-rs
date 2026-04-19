#![allow(unused_unsafe)]
#![allow(unsafe_code)]

mod crc32custom;
mod level;
mod log_parse;
mod log_parser;
mod log_parser_node;
mod log_parser_parse;
mod log_parser_tree_builder;
mod log_rend;
mod log_rend_json;
mod log_render;
mod log_render_color;
mod log_render_json;
mod log_render_tree;
mod log_render_tree_prefixes;
mod log_transfomer_into_json;
mod log_transfomer_into_json_consts;
mod slice_items;
mod test;
mod transform_json_items;
mod value_kind;
mod pointer_ext;
mod itoa4;
mod itoa2;

pub use log_parser::LogParser;
pub use log_render::LogRender;
