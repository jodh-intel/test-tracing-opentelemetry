// Copyright (c) 2020 Intel Corporation
//
// SPDX-License-Identifier: Apache-2.0
//

use async_trait::async_trait;
use nix::sys::socket::{SockAddr, VsockAddr};
use opentelemetry::sdk::export::trace::{ExportResult, SpanData, SpanExporter};
use opentelemetry::sdk::export::ExportError;
use slog::{error, o, Logger};
use std::io;
use std::io::Write;
use std::net::Shutdown;
use std::sync::Mutex;
use vsock::VsockStream;

const ANY_CID: &'static str = "any";

// reserved for "host"
const DEFAULT_CID: u32 = 2;
const DEFAULT_PORT: u32 = 10240;

#[derive(Debug)]
pub struct Exporter {
    port: u32,
    cid: u32,
    conn: Mutex<VsockStream>,
    logger: Logger,
}

#[derive(Debug)]
pub struct VSockExporterError(String);

impl Exporter {
    // Create a new exporter builder.
    pub fn builder() -> Builder {
        Builder::default()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("connection error: {0}")]
    ConnectionError(String),
    #[error("serialisation error: {0}")]
    SerialisationError(#[from] bincode::Error),
    #[error("I/O error: {0}")]
    IOError(#[from] io::Error),
}

impl ExportError for Error {
    fn exporter_name(&self) -> &'static str {
        "vsock-exporter"
    }
}

#[async_trait]
impl SpanExporter for Exporter {
    async fn export(&mut self, batch: Vec<SpanData>) -> ExportResult {
        let cid_str: String;
        if self.cid == libc::VMADDR_CID_ANY {
            cid_str = ANY_CID.to_string();
        } else {
            cid_str = format!("{}", self.cid);
        }

        let logger = self.logger.new(o!("cid" => cid_str, "port" => self.port));

        let mut conn = match self.conn.lock() {
            Ok(conn) => conn,
            Err(e) => {
                error!(logger, "failed to obtain connection";
                        "error" => format!("{}", e));

                return Err(Error::ConnectionError(e.to_string()).into());
            }
        };

        for entry in batch {
            let span_data = entry;

            let encoded: Vec<u8> = match bincode::serialize(&span_data) {
                Ok(d) => d,
                Err(e) => {
                    error!(logger, "failed to serialise trace spans";
                        "error" => format!("{}", e));

                    return Err(Error::SerialisationError(e).into());
                }
            };

            if let Err(e) = conn.write(&encoded) {
                error!(logger, "failed to write serialised trace spans";
                        "error" => format!("{}", e));

                return Err(Error::IOError(e).into());
            }
        }

        Ok(())
    }

    // Ignored for now.
    fn shutdown(&mut self) {
        let conn = match self.conn.lock() {
            Ok(conn) => conn,
            Err(e) => {
                error!(self.logger, "failed to obtain connection";
                        "error" => format!("{}", e));
                return;
            }
        };

        conn.shutdown(Shutdown::Write)
            .expect("failed to shutdown VSOCK connection");
    }
}

#[derive(Debug)]
pub struct Builder {
    port: u32,
    cid: u32,
    logger: Logger,
}

impl Default for Builder {
    fn default() -> Self {
        let logger = Logger::root(slog::Discard, o!());

        Builder {
            cid: DEFAULT_CID,
            port: DEFAULT_PORT,
            logger: logger,
        }
    }
}

impl Builder {
    pub fn with_cid(self, cid: u32) -> Self {
        Builder { cid, ..self }
    }

    pub fn with_port(self, port: u32) -> Self {
        Builder { port, ..self }
    }

    pub fn with_logger(self, logger: &Logger) -> Self {
        Builder {
            logger: logger.new(o!()),
            ..self
        }
    }

    pub fn init(self) -> Exporter {
        let Builder { port, cid, logger } = self;

        let vsock_addr = VsockAddr::new(self.cid, self.port);
        let sock_addr = SockAddr::Vsock(vsock_addr);

        let cid_str: String;

        if self.cid == libc::VMADDR_CID_ANY {
            cid_str = ANY_CID.to_string();
        } else {
            cid_str = format!("{}", self.cid);
        }

        let conn = VsockStream::connect(&sock_addr).expect(&format!(
            "failed to connect to VSOCK server (port: {}, cid: {}) - {}",
            self.port, cid_str, "ensure trace forwarder is running on host"
        ));

        Exporter {
            port: port,
            cid: cid,
            conn: Mutex::new(conn),
            logger: logger.new(o!()),
        }
    }
}
