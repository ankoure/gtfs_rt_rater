pub mod fetch;
pub mod output;
pub mod parser;
pub mod stats;

pub mod gtfs_rt {
    include!(concat!(env!("OUT_DIR"), "/transit_realtime.rs"));
}
