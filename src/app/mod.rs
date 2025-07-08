use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use std::{fmt, io};

use config::Config;
use tokio::net::TcpListener;
use tokio::time::sleep;

mod config;
mod hardware_addr;
mod upstream;

use self::upstream::Upstream;
