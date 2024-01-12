#[macro_use]
extern crate lazy_static;

use async_compression::tokio::write::BrotliEncoder;
use async_compression::tokio::write::GzipEncoder;
use async_compression::Level;
use base64::Engine;
use cache_control::CacheControl;
use deno_core::error::custom_error;
use deno_core::error::AnyError;

use crate::reader_stream::ExternallyAbortableReaderStream;
use crate::reader_stream::ShutdownHandle;
use deno_core::futures::stream::Peekable;
use deno_core::futures::FutureExt;
use deno_core::futures::StreamExt;
use deno_core::futures::TryFutureExt;
use deno_core::futures::{ready, SinkExt};
use deno_core::op2;
use deno_core::AsyncRefCell;
use deno_core::AsyncResult;
use deno_core::BufView;
use deno_core::ByteString;
use deno_core::CancelFuture;
use deno_core::CancelHandle;
use deno_core::CancelTryFuture;
use deno_core::JsBuffer;
use deno_core::Op;
use deno_core::OpState;
use deno_core::RcRef;
use deno_core::Resource;
use deno_core::ResourceId;
use deno_core::StringOrBuffer;
use flate2::write::GzEncoder;
use flate2::Compression;
use fly_accept_encoding::Encoding;
use hyper::body::Bytes;
use hyper::body::HttpBody;
use hyper::body::SizeHint;
use hyper::header::HeaderName;
use hyper::header::HeaderValue;
use hyper::service::Service;
use hyper::Body;
use hyper::HeaderMap;
use hyper::Request;
use hyper::Response;
use hyper::StatusCode;
use serde::Serialize;
use std::borrow::Cow;
use std::cell::RefCell;
use std::cmp::min;
use std::error::Error;
use std::future::Future;
use std::io;
use std::io::Write;
use std::mem::replace;
use std::mem::take;
use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::{select, time};

pub mod compressible;
mod reader_stream;

pub type HttpSender = async_channel::Sender<RequestContext>;
pub type HttpReceiver = async_channel::Receiver<RequestContext>;


pub struct RequestContext {
    pub request: Request<Body>,
    pub response_tx: mpsc::Sender<hyper::Response<Body>>,
}
/// 覆盖 deno_http ext中的以下几个op
/// op_http_accept  接受请求
/// op_http_headers 获取header
/// op_http_write_headers 写入header
/// op_http_write_resource 写入
/// op_http_write 写入
/// 替换 request: Request<Body> 的获取来源 需要用户自定义server 使用 REQUEST_CHANNEL.0 发送请求，
/// 去掉资源的主动关闭
deno_core::extension!(
  simple_http,
  deps = [deno_fetch],
  esm_entry_point = "ext:simple_http/simplehttp.js",
  esm = ["simplehttp.js"],
      options = {
    recever: HttpReceiver,
  },
  middleware = |op| match op.name {
    "op_http_accept" => op_http_accept::DECL,
    "op_http_headers" => op_http_headers::DECL,
    "op_http_write_headers" => op_http_write_headers::DECL,
    "op_http_write_resource" => op_http_write_resource::DECL,
    "op_http_write" => op_http_write::DECL,
    "op_http_shutdown" => op_http_shutdown::DECL,
    _ => op,
  },
      state = |state, options| {
    state.put(options.recever);
  },

);

/// A resource representing a single HTTP request/response stream.
pub struct HttpStreamResource {
    pub rd: AsyncRefCell<HttpRequestReader>,
    wr: AsyncRefCell<HttpResponseWriter>,
    accept_encoding: Encoding,
    cancel_handle: CancelHandle,
    size: SizeHint,
}

impl HttpStreamResource {
    fn new(request: Request<Body>, response_tx: mpsc::Sender<Response<Body>>, accept_encoding: Encoding) -> Self {
        let size = request.body().size_hint();
        Self {
            rd: HttpRequestReader::Headers(request).into(),
            wr: HttpResponseWriter::Headers(response_tx).into(),
            accept_encoding,
            size,
            cancel_handle: CancelHandle::new(),
        }
    }
}

impl Resource for HttpStreamResource {
    fn name(&self) -> Cow<str> {
        "httpStream".into()
    }

    fn read(self: Rc<Self>, limit: usize) -> AsyncResult<BufView> {
        Box::pin(async move {
            let mut rd = RcRef::map(&self, |r| &r.rd).borrow_mut().await;

            let body = loop {
                match &mut *rd {
                    HttpRequestReader::Headers(_) => {}
                    HttpRequestReader::Body(_, body) => break body,
                    HttpRequestReader::Closed => return Ok(BufView::empty()),
                }
                match take(&mut *rd) {
                    HttpRequestReader::Headers(request) => {
                        let (parts, body) = request.into_parts();
                        *rd = HttpRequestReader::Body(parts.headers, body.peekable());
                    }
                    _ => unreachable!(),
                };
            };

            let fut = async {
                let mut body = Pin::new(body);
                loop {
                    match body.as_mut().peek_mut().await {
                        Some(Ok(chunk)) if !chunk.is_empty() => {
                            let len = min(limit, chunk.len());
                            let buf = chunk.split_to(len);
                            let view = BufView::from(buf);
                            break Ok(view);
                        }
                        // This unwrap is safe because `peek_mut()` returned `Some`, and thus
                        // currently has a peeked value that can be synchronously returned
                        // from `next()`.
                        //
                        // The future returned from `next()` is always ready, so we can
                        // safely call `await` on it without creating a race condition.
                        Some(_) => match body.as_mut().next().await.unwrap() {
                            Ok(chunk) => assert!(chunk.is_empty()),
                            Err(err) => break Err(AnyError::from(err)),
                        },
                        None => break Ok(BufView::empty()),
                    }
                }
            };

            let cancel_handle = RcRef::map(&self, |r| &r.cancel_handle);
            fut.try_or_cancel(cancel_handle).await
        })
    }

    fn close(self: Rc<Self>) {
        self.cancel_handle.cancel();
    }

    fn size_hint(&self) -> (u64, Option<u64>) {
        (self.size.lower(), self.size.upper())
    }
}

/// The read half of an HTTP stream.
pub enum HttpRequestReader {
    Headers(Request<Body>),
    Body(HeaderMap<HeaderValue>, Peekable<Body>),
    Closed,
}

impl Default for HttpRequestReader {
    fn default() -> Self {
        Self::Closed
    }
}

/// The write half of an HTTP stream.
enum HttpResponseWriter {
    Headers(mpsc::Sender<Response<Body>>),
    Body { writer: Pin<Box<dyn tokio::io::AsyncWrite>>, shutdown_handle: ShutdownHandle },
    BodyUncompressed(BodyUncompressedSender),
    Closed,
}

impl Default for HttpResponseWriter {
    fn default() -> Self {
        Self::Closed
    }
}

struct BodyUncompressedSender(Option<hyper::body::Sender>);

impl BodyUncompressedSender {
    fn sender(&mut self) -> &mut hyper::body::Sender {
        // This is safe because we only ever take the sender out of the option
        // inside of the shutdown method.
        self.0.as_mut().unwrap()
    }

    fn shutdown(mut self) {
        // take the sender out of self so that when self is dropped at the end of
        // this block, it doesn't get aborted
        self.0.take();
    }
}

impl From<hyper::body::Sender> for BodyUncompressedSender {
    fn from(sender: hyper::body::Sender) -> Self {
        BodyUncompressedSender(Some(sender))
    }
}

impl Drop for BodyUncompressedSender {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            sender.abort();
        }
    }
}

// We use a tuple instead of struct to avoid serialization overhead of the keys.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NextRequestResponse(
    // stream_rid:
    ResourceId,
    // method:
    // This is a String rather than a ByteString because reqwest will only return
    // the method as a str which is guaranteed to be ASCII-only.
    String,
    // url:
    String,
);

#[op2(async)]
#[serde]
async fn op_http_accept(state: Rc<RefCell<OpState>>, #[smi] _rid: ResourceId) -> Result<Option<NextRequestResponse>, AnyError> {
    let receiver = { state.borrow().borrow::<HttpReceiver>().clone() };

    match receiver.recv().await {
        Ok(item) => {
            let RequestContext { request, response_tx, .. } = item;
            let (stream, method, url) = build_http_stream_resource("http", request, response_tx);
            let stream_rid = state.borrow_mut().resource_table.add(stream);
            let r = NextRequestResponse(stream_rid, method, url);
            Ok(Some(r))
        }
        Err(err) => Err(AnyError::from(err)),
    }
}

pub fn build_http_stream_resource(scheme: &'static str, request: Request<Body>, response_tx: mpsc::Sender<hyper::Response<Body>>) -> (HttpStreamResource, String, String) {
    let accept_encoding = {
        let encodings = fly_accept_encoding::encodings_iter(request.headers()).filter(|r| matches!(r, Ok((Some(Encoding::Brotli | Encoding::Gzip), _))));

        fly_accept_encoding::preferred(encodings).ok().flatten().unwrap_or(Encoding::Identity)
    };
    let method = request.method().to_string();
    let url = req_url(&request, scheme);
    let stream = HttpStreamResource::new(request, response_tx, accept_encoding);
    (stream, method, url)
}

fn req_url(req: &hyper::Request<Body>, scheme: &'static str) -> String {
    let mut host = "127.0.0.1";
    if req.headers().get("host").is_some() {
        host = req.headers().get("host").unwrap().to_str().unwrap();
    }

    let path = req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/");
    [scheme, "://", &host, path].concat()
}

fn req_headers(header_map: &HeaderMap<HeaderValue>) -> Vec<(ByteString, ByteString)> {
    // We treat cookies specially, because we don't want them to get them
    // mangled by the `Headers` object in JS. What we do is take all cookie
    // headers and concat them into a single cookie header, separated by
    // semicolons.
    let cookie_sep = "; ".as_bytes();
    let mut cookies = vec![];

    let mut headers = Vec::with_capacity(header_map.len());
    for (name, value) in header_map.iter() {
        if name == hyper::header::COOKIE {
            cookies.push(value.as_bytes());
        } else {
            let name: &[u8] = name.as_ref();
            let value = value.as_bytes();
            headers.push((name.into(), value.into()));
        }
    }

    if !cookies.is_empty() {
        headers.push(("cookie".into(), cookies.join(cookie_sep).into()));
    }

    headers
}

#[op2(async)]
async fn op_http_write_headers(state: Rc<RefCell<OpState>>, #[smi] rid: u32, #[smi] status: u16, #[serde] headers: Vec<(ByteString, ByteString)>, #[serde] data: Option<StringOrBuffer>) -> Result<(), AnyError> {
    let stream = state.borrow_mut().resource_table.get::<HttpStreamResource>(rid)?;

    // Track supported encoding
    let encoding = stream.accept_encoding;

    let mut builder = Response::builder();
    // SAFETY: can not fail, since a fresh Builder is non-errored
    let hmap = unsafe { builder.headers_mut().unwrap_unchecked() };

    // Add headers
    hmap.reserve(headers.len() + 2);
    for (k, v) in headers.into_iter() {
        let v: Vec<u8> = v.into();
        hmap.append(HeaderName::try_from(k.as_slice())?, HeaderValue::try_from(v)?);
    }
    ensure_vary_accept_encoding(hmap);

    let accepts_compression = matches!(encoding, Encoding::Brotli | Encoding::Gzip);
    let compressing = accepts_compression && (matches!(data, Some(ref data) if data.len() > 20) || data.is_none()) && should_compress(hmap);

    if compressing {
        weaken_etag(hmap);
        // Drop 'content-length' header. Hyper will update it using compressed body.
        hmap.remove(hyper::header::CONTENT_LENGTH);
        // Content-Encoding header
        hmap.insert(
            hyper::header::CONTENT_ENCODING,
            HeaderValue::from_static(match encoding {
                Encoding::Brotli => "br",
                Encoding::Gzip => "gzip",
                _ => unreachable!(), // Forbidden by accepts_compression
            }),
        );
    }

    let (new_wr, body) = http_response(data, compressing, encoding)?;
    let body = builder.status(status).body(body)?;

    let mut old_wr = RcRef::map(&stream, |r| &r.wr).borrow_mut().await;
    let mut response_tx = match replace(&mut *old_wr, new_wr) {
        HttpResponseWriter::Headers(response_tx) => response_tx,
        _ => return Err(http_error("response headers already sent")),
    };

    match response_tx.send(body).await {
        Ok(_) => Ok(()),
        Err(_) => Err(http_error("connection closed while sending response")),
    }
}

#[op2]
#[serde]
fn op_http_headers(state: &mut OpState, #[smi] rid: u32) -> Result<Vec<(ByteString, ByteString)>, AnyError> {
    let stream = state.resource_table.get::<HttpStreamResource>(rid)?;
    let rd = RcRef::map(&stream, |r| &r.rd).try_borrow().ok_or_else(|| http_error("already in use"))?;
    match &*rd {
        HttpRequestReader::Headers(request) => Ok(req_headers(request.headers())),
        HttpRequestReader::Body(headers, _) => Ok(req_headers(headers)),
        _ => unreachable!(),
    }
}

fn http_response(data: Option<StringOrBuffer>, compressing: bool, encoding: Encoding) -> Result<(HttpResponseWriter, hyper::Body), AnyError> {
    // Gzip, after level 1, doesn't produce significant size difference.
    // This default matches nginx default gzip compression level (1):
    // https://nginx.org/en/docs/http/ngx_http_gzip_module.html#gzip_comp_level
    const GZIP_DEFAULT_COMPRESSION_LEVEL: u8 = 1;

    match data {
        Some(data) if compressing => match encoding {
            Encoding::Brotli => {
                // quality level 6 is based on google's nginx default value for
                // on-the-fly compression
                // https://github.com/google/ngx_brotli#brotli_comp_level
                // lgwin 22 is equivalent to brotli window size of (2**22)-16 bytes
                // (~4MB)
                let mut writer = brotli::CompressorWriter::new(Vec::new(), 4096, 6, 22);
                writer.write_all(&data)?;
                Ok((HttpResponseWriter::Closed, writer.into_inner().into()))
            }
            Encoding::Gzip => {
                let mut writer = GzEncoder::new(Vec::new(), Compression::new(GZIP_DEFAULT_COMPRESSION_LEVEL.into()));
                writer.write_all(&data)?;
                Ok((HttpResponseWriter::Closed, writer.finish()?.into()))
            }
            _ => unreachable!(), // forbidden by accepts_compression
        },
        Some(data) => {
            // If a buffer was passed, but isn't compressible, we use it to
            // construct a response body.
            Ok((HttpResponseWriter::Closed, Bytes::from(data).into()))
        }
        None if compressing => {
            // Create a one way pipe that implements tokio's async io traits. To do
            // this we create a [tokio::io::DuplexStream], but then throw away one
            // of the directions to create a one way pipe.
            let (a, b) = tokio::io::duplex(64 * 1024);
            let (reader, _) = tokio::io::split(a);
            let (_, writer) = tokio::io::split(b);
            let writer: Pin<Box<dyn tokio::io::AsyncWrite>> = match encoding {
                Encoding::Brotli => Box::pin(BrotliEncoder::with_quality(writer, Level::Fastest)),
                Encoding::Gzip => Box::pin(GzipEncoder::with_quality(writer, Level::Precise(GZIP_DEFAULT_COMPRESSION_LEVEL.into()))),
                _ => unreachable!(), // forbidden by accepts_compression
            };
            let (stream, shutdown_handle) = ExternallyAbortableReaderStream::new(reader);
            Ok((HttpResponseWriter::Body { writer, shutdown_handle }, Body::wrap_stream(stream)))
        }
        None => {
            let (body_tx, body_rx) = Body::channel();
            Ok((HttpResponseWriter::BodyUncompressed(body_tx.into()), body_rx))
        }
    }
}

// If user provided a ETag header for uncompressed data, we need to
// ensure it is a Weak Etag header ("W/").
fn weaken_etag(hmap: &mut hyper::HeaderMap) {
    if let Some(etag) = hmap.get_mut(hyper::header::ETAG) {
        if !etag.as_bytes().starts_with(b"W/") {
            let mut v = Vec::with_capacity(etag.as_bytes().len() + 2);
            v.extend(b"W/");
            v.extend(etag.as_bytes());
            *etag = v.try_into().unwrap();
        }
    }
}

// Set Vary: Accept-Encoding header for direct body response.
// Note: we set the header irrespective of whether or not we compress the data
// to make sure cache services do not serve uncompressed data to clients that
// support compression.
fn ensure_vary_accept_encoding(hmap: &mut hyper::HeaderMap) {
    if let Some(v) = hmap.get_mut(hyper::header::VARY) {
        if let Ok(s) = v.to_str() {
            if !s.to_lowercase().contains("accept-encoding") {
                *v = format!("Accept-Encoding, {s}").try_into().unwrap()
            }
            return;
        }
    }
    hmap.insert(hyper::header::VARY, HeaderValue::from_static("Accept-Encoding"));
}

///根据header 判断是否需要压缩
fn should_compress(headers: &hyper::HeaderMap) -> bool {
    // 如果缓存控制标头值设置为“无转换”或不是utf8，则跳过压缩
    fn cache_control_no_transform(headers: &hyper::HeaderMap) -> Option<bool> {
        let v = headers.get(hyper::header::CACHE_CONTROL)?;
        let s = match std::str::from_utf8(v.as_bytes()) {
            Ok(s) => s,
            Err(_) => return Some(true),
        };
        let c = CacheControl::from_value(s)?;
        Some(c.no_transform)
    }
    //如果设置了“content-range”标头值，则跳过压缩
    //表示正文的内容是直接协商的
    //使用用户代码，我们无法压缩响应
    let content_range = headers.contains_key(hyper::header::CONTENT_RANGE);
    // 如果存在Content-Encoding标头，则假定正文已被压缩，从而避免重新压缩
    let is_precompressed = headers.contains_key(hyper::header::CONTENT_ENCODING);

    !content_range && !is_precompressed && !cache_control_no_transform(headers).unwrap_or_default() && headers.get(hyper::header::CONTENT_TYPE).map(compressible::is_content_compressible).unwrap_or_default()
}

#[op2(async)]
async fn op_http_write_resource(state: Rc<RefCell<OpState>>, #[smi] rid: ResourceId, #[smi] stream: ResourceId) -> Result<(), AnyError> {
    let http_stream = state.borrow().resource_table.get::<HttpStreamResource>(rid)?;
    let mut wr = RcRef::map(&http_stream, |r| &r.wr).borrow_mut().await;
    let resource = state.borrow().resource_table.get_any(stream)?;
    loop {
        match *wr {
            HttpResponseWriter::Headers(_) => return Err(http_error("no response headers")),
            HttpResponseWriter::Closed => return Err(http_error("response already completed")),
            _ => {}
        };

        let view = resource.clone().read(64 * 1024).await?; // 64KB
        if view.is_empty() {
            break;
        }

        match &mut *wr {
            HttpResponseWriter::Body { writer, .. } => {
                let mut result = writer.write_all(&view).await;
                if result.is_ok() {
                    result = writer.flush().await;
                }
                if let Err(err) = result {
                    assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
                    // If there was no connection error, drop body_tx.
                    *wr = HttpResponseWriter::Closed;
                }
            }
            HttpResponseWriter::BodyUncompressed(body) => {
                let bytes = Bytes::from(view);
                if let Err(err) = body.sender().send_data(bytes).await {
                    assert!(err.is_closed());
                    // If there was no connection error, drop body_tx.
                    *wr = HttpResponseWriter::Closed;
                }
            }
            _ => unreachable!(),
        };
    }
    Ok(())
}

#[op2(async)]
async fn op_http_write(state: Rc<RefCell<OpState>>, #[smi] rid: ResourceId, #[buffer] buf: JsBuffer) -> Result<(), AnyError> {
    let stream = state.borrow().resource_table.get::<HttpStreamResource>(rid)?;
    let mut wr = RcRef::map(&stream, |r| &r.wr).borrow_mut().await;

    match &mut *wr {
        HttpResponseWriter::Headers(_) => Err(http_error("no response headers")),
        HttpResponseWriter::Closed => Err(http_error("response already completed")),
        HttpResponseWriter::Body { writer, .. } => {
            let mut result = writer.write_all(&buf).await;
            if result.is_ok() {
                result = writer.flush().await;
            }
            match result {
                Ok(_) => Ok(()),
                Err(err) => {
                    assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
                    *wr = HttpResponseWriter::Closed;
                    Err(http_error("response already completed"))
                }
            }
        }
        HttpResponseWriter::BodyUncompressed(body) => {
            let bytes = Bytes::from(buf);
            match body.sender().send_data(bytes).await {
                Ok(_) => Ok(()),
                Err(err) => {
                    assert!(err.is_closed());
                    // Pull up the failure associated with the transport connection instead.
                    // If there was no connection error, drop body_tx.
                    *wr = HttpResponseWriter::Closed;
                    Err(http_error("response already completed"))
                }
            }
        }
    }
}

#[op2(async)]
async fn op_http_shutdown(state: Rc<RefCell<OpState>>, #[smi] rid: ResourceId) -> Result<(), AnyError> {
    let stream = state.borrow().resource_table.get::<HttpStreamResource>(rid)?;
    let mut wr = RcRef::map(&stream, |r| &r.wr).borrow_mut().await;
    let wr = take(&mut *wr);
    match wr {
        HttpResponseWriter::Body { mut writer, shutdown_handle } => {
            shutdown_handle.shutdown();
            match writer.shutdown().await {
                Ok(_) => {}
                Err(err) => {}
            }
        }
        HttpResponseWriter::BodyUncompressed(body) => {
            body.shutdown();
        }
        _ => {}
    };
    Ok(())
}

fn http_error(message: &'static str) -> AnyError {
    custom_error("Http", message)
}

