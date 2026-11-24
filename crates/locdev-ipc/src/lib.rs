//! Local IPC between clients (CLI, desktop) and the daemon.
//!
//! Messages are JSON [`Envelope`]s framed with a 4-byte length prefix. The
//! transport is a Unix domain socket (mode 0600, inside a 0700 directory) on
//! Unix and a per-user named pipe on Windows that rejects remote clients.

pub mod launcher;
pub mod transport;

use std::future::Future;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use locdev_core::paths::Paths;
use locdev_core::protocol::{Envelope, PROTOCOL_VERSION, Request, Response};
use locdev_core::{AppError, ErrorCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::bytes::Bytes;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub use transport::{Endpoint, Listener};

/// Upper bound for a single message; log batches are the largest payloads.
const MAX_FRAME: usize = 8 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("malformed message: {0}")]
    Codec(#[from] serde_json::Error),
    #[error("connection closed by peer")]
    Closed,
    #[error("daemon speaks protocol {daemon}, this client speaks {client}")]
    IncompatibleProtocol { daemon: u32, client: u32 },
    #[error("timed out waiting for the daemon")]
    Timeout,
    #[error(transparent)]
    App(#[from] AppError),
}

fn framed<S: AsyncRead + AsyncWrite>(stream: S) -> Framed<S, LengthDelimitedCodec> {
    let codec = LengthDelimitedCodec::builder()
        .max_frame_length(MAX_FRAME)
        .new_codec();
    Framed::new(stream, codec)
}

async fn send<S, T>(framed: &mut Framed<S, LengthDelimitedCodec>, body: T) -> Result<(), IpcError>
where
    S: AsyncRead + AsyncWrite + Unpin,
    T: Serialize,
{
    let bytes = serde_json::to_vec(&Envelope::new(body))?;
    framed.send(Bytes::from(bytes)).await?;
    Ok(())
}

async fn recv<S, T>(
    framed: &mut Framed<S, LengthDelimitedCodec>,
) -> Result<Option<Envelope<T>>, IpcError>
where
    S: AsyncRead + AsyncWrite + Unpin,
    T: DeserializeOwned,
{
    match framed.next().await {
        Some(frame) => Ok(Some(serde_json::from_slice(&frame?)?)),
        None => Ok(None),
    }
}

/// Serves requests on one connection until the client disconnects.
pub async fn serve<S, H, F>(stream: S, handler: H) -> Result<(), IpcError>
where
    S: AsyncRead + AsyncWrite + Unpin,
    H: Fn(Request) -> F,
    F: Future<Output = Response>,
{
    let mut framed = framed(stream);

    loop {
        let envelope = match recv::<_, serde_json::Value>(&mut framed).await {
            Ok(Some(envelope)) => envelope,
            Ok(None) => return Ok(()),
            Err(IpcError::Codec(error)) => {
                let error = AppError::invalid(format!("malformed request: {error}"));
                send(&mut framed, Response::Error(error)).await?;
                continue;
            }
            Err(error) => return Err(error),
        };

        // Check the version before decoding the body: a newer client may send
        // request types this daemon does not know about.
        let response = if envelope.protocol != PROTOCOL_VERSION {
            Response::Error(AppError::new(
                ErrorCode::IncompatibleProtocol,
                format!(
                    "client speaks protocol {}, daemon speaks {PROTOCOL_VERSION}",
                    envelope.protocol
                ),
            ))
        } else {
            match serde_json::from_value::<Request>(envelope.body) {
                Ok(request) => handler(request).await,
                Err(error) => {
                    Response::Error(AppError::invalid(format!("unknown request: {error}")))
                }
            }
        };

        send(&mut framed, response).await?;
    }
}

pub struct Client<S> {
    framed: Framed<S, LengthDelimitedCodec>,
}

impl<S: AsyncRead + AsyncWrite + Unpin> Client<S> {
    pub fn new(stream: S) -> Self {
        Self {
            framed: framed(stream),
        }
    }

    /// Sends one request and waits for its response. Daemon-side failures
    /// are returned as [`IpcError::App`].
    pub async fn call(&mut self, request: Request) -> Result<Response, IpcError> {
        send(&mut self.framed, request).await?;

        let envelope = recv::<_, Response>(&mut self.framed)
            .await?
            .ok_or(IpcError::Closed)?;

        if envelope.protocol != PROTOCOL_VERSION {
            return Err(IpcError::IncompatibleProtocol {
                daemon: envelope.protocol,
                client: PROTOCOL_VERSION,
            });
        }

        Ok(envelope.body.into_result()?)
    }
}

pub type LocalClient = Client<transport::ClientStream>;

/// Connects to the daemon for the given paths.
pub async fn connect(paths: &Paths) -> Result<LocalClient, IpcError> {
    let stream = Endpoint::for_paths(paths).connect().await?;
    Ok(Client::new(stream))
}

/// Connects and sends a single request, with a timeout.
pub async fn request(paths: &Paths, request: Request) -> Result<Response, IpcError> {
    tokio::time::timeout(Duration::from_secs(30), async {
        connect(paths).await?.call(request).await
    })
    .await
    .map_err(|_| IpcError::Timeout)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use locdev_core::model::{DaemonStatus, ProjectView};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn handler(request: Request) -> Response {
        match request {
            Request::Status => Response::Status(DaemonStatus {
                version: "test".into(),
                protocol: PROTOCOL_VERSION,
                pid: 1,
                started_at: 0,
                projects: 0,
                running: 0,
                home: "/tmp".into(),
                proxy: Default::default(),
                dns: Default::default(),
                tls: Default::default(),
                mail: Default::default(),
                unread_notifications: 0,
            }),
            Request::RemoveProject { name } => {
                Response::Error(AppError::not_found(format!("no project named {name}")))
            }
            _ => Response::Projects {
                projects: Vec::<ProjectView>::new(),
            },
        }
    }

    #[tokio::test]
    async fn client_receives_status() {
        let (client, server) = tokio::io::duplex(8192);
        let task = tokio::spawn(serve(server, handler));

        let mut client = Client::new(client);
        let response = client.call(Request::Status).await.unwrap();
        assert!(matches!(
            response,
            Response::Status(DaemonStatus { pid: 1, .. })
        ));

        // The same connection keeps serving requests.
        let response = client.call(Request::ListProjects).await.unwrap();
        assert!(matches!(response, Response::Projects { .. }));

        drop(client);
        task.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn daemon_errors_become_app_errors() {
        let (client, server) = tokio::io::duplex(8192);
        tokio::spawn(serve(server, handler));

        let error = Client::new(client)
            .call(Request::RemoveProject { name: "x".into() })
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            IpcError::App(AppError {
                code: ErrorCode::NotFound,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn rejects_incompatible_protocol() {
        let (client, server) = tokio::io::duplex(8192);
        tokio::spawn(serve(server, handler));

        let mut framed = framed(client);
        let message = serde_json::json!({ "protocol": 999, "body": { "type": "from_the_future" } });
        framed
            .send(Bytes::from(serde_json::to_vec(&message).unwrap()))
            .await
            .unwrap();

        let reply: Envelope<Response> = recv(&mut framed).await.unwrap().unwrap();
        assert!(matches!(
            reply.body,
            Response::Error(AppError {
                code: ErrorCode::IncompatibleProtocol,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn malformed_frames_do_not_kill_the_connection() {
        let (mut client, server) = tokio::io::duplex(8192);
        tokio::spawn(serve(server, handler));

        let garbage = b"not json";
        client.write_u32(garbage.len() as u32).await.unwrap();
        client.write_all(garbage).await.unwrap();

        let len = client.read_u32().await.unwrap();
        let mut reply = vec![0; len as usize];
        client.read_exact(&mut reply).await.unwrap();
        let reply: Envelope<Response> = serde_json::from_slice(&reply).unwrap();
        assert!(matches!(
            reply.body,
            Response::Error(AppError {
                code: ErrorCode::InvalidInput,
                ..
            })
        ));

        let response = Client::new(client).call(Request::Status).await.unwrap();
        assert!(matches!(response, Response::Status(_)));
    }

    #[tokio::test]
    async fn local_endpoint_round_trip() {
        let home = tempfile::tempdir().unwrap();
        let paths = Paths::under(home.path().to_path_buf());
        paths.ensure().unwrap();

        let mut listener = Listener::bind(&Endpoint::for_paths(&paths)).await.unwrap();
        tokio::spawn(async move {
            loop {
                let stream = listener.accept().await.unwrap();
                tokio::spawn(serve(stream, handler));
            }
        });

        let response = request(&paths, Request::Status).await.unwrap();
        assert!(matches!(response, Response::Status(_)));

        // A second daemon must refuse to take over a live endpoint.
        let Err(error) = Listener::bind(&Endpoint::for_paths(&paths)).await else {
            panic!("second listener bound to a live endpoint");
        };
        assert_eq!(error.kind(), std::io::ErrorKind::AddrInUse);
    }
}
