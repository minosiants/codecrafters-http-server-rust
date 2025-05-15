use std::future::Future;
use std::sync::Arc;

use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, ToSocketAddrs};

use crate::{Complete, Connection, Context, Error, Request, Result, State};

pub struct Server {
    listener: TcpListener,
}

impl Server {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> Result<Server> {
        let listener = TcpListener::bind(addr)
            .await
            .with_context(|| "bind connection")?;
        Ok(Server { listener })
    }
}

pub trait Serve {
    async fn serve<F, Fut>(self, f: Arc<F>) -> Result<()>
    where
        F: Fn(State) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<State>> + Send;
}

impl Serve for Server {
    async fn serve<F, Fut>(self, f: Arc<F>) -> Result<()>
    where
        F: Fn(State) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<State>> + Send,
    {
        loop {
            let (mut stream, _) = self.listener.accept().await.with_context(|| "")?;
            let f_cloned = Arc::clone(&f);
            tokio::spawn(async move {
                loop {
                    let request = Request::read(&mut stream).await?;
                    let state = f_cloned(State::incomplete(Arc::new(request))).await?;
                    match state {
                        State::Incomplete(_) => {}
                        State::Complete(Complete(req, resp)) => {
                            let bytes: Vec<u8> = {
                                resp.borrow().clone().into()
                            };
                            stream.write_all(bytes.as_ref()).await.with_context(|| "")?;
                            stream.flush().await.with_context(|| "flushing ")?;
                            if req.headers.connection() == Some(Connection::Close) {
                                break;
                            }
                        }
                    }
                }
                Ok::<(), Error>(())
            });
        }
    }
}
