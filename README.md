
# CodeCrafters http server in rust

My primary development expertise lies in JVM-based world. I began with Java and then fortunately
moved to Scala, and FP. Since then I've been infected with FP :) . Unfortunately, despite Scala is great, its popularity has
significantly declined in recent years. In Australia, for instance, Scala job opportunities are virtually nonexistent.
Java is not hot anymore as it used to be thankfully Typescript. Personally, I find coding in Java far less exciting than working 
with languages that have better FP support. Without a heavy use features like algebraic data types (ADTs), newtypes, pattern matching, and functional 
composition when you relay a lot on a compiler is difficult to write maintainable and less buggy code.

Last year, I began exploring Rust and subscribed to https://codecrafters.io/ to deepen my learning. While the subscription is
somewhat pricey, the platform offers valuable hands-on challenges. I took the first challenge as [codecrafters-kafka-rust](https://github.com/minosiants/codecrafters-kafka-rust)
challenge, which helped me as an excellent introduction to Rust’s syntax and core concepts. More recently, I completed the
[codecrafters-http-server-rust](https://github.com/minosiants/codecrafters-http-server-rust) challenge.
After I finished and all CodeCrafters tests were happy I decided to refactor it to more functional style than it was.

## ["Build Your Own HTTP server" Challenge](https://app.codecrafters.io/courses/http-server/overview).


![http-server-tasks.png](media%2Fhttp-server-tasks.png)

First, I want to present the results and then shed some light on them.

```rust

#[tokio::main]
async fn main() -> Result<()> {
    let server = Server::bind("127.0.0.1:4221").await?;
    server
        .serve(Arc::new(async move |state| {
            routes().handle(state).map(|v| v.0)
        }))
        .await
}

pub fn routes() -> impl Endpoint<Output=UnitT> {
    let v = user_agent()
        .or(route::get("/echo").set_response(path().flat_map(|v| ok(v))))
        .or(get_file())
        .or(post_file())
        .or(route::get("/").set_response(ok("")))
        .or(state().set_response(not_found("")))
        .and(gzip().and(close_connection()))
        .unit();

    v
}
fn user_agent() -> impl Endpoint<Output=UnitT> {
    let response = |v: Option<UserAgent>| ok(v.map(|v| v.0).unwrap_or("".to_string()));

    route::get("/user-agent").set_response(get_user_agent().flat_map(response))
}

fn get_file() -> impl Endpoint<Output=UnitT> {
    let read = |file: String| {
        lift(
            match format!("/tmp/data/codecrafters.io/http-server-tester/{}", file)
                .as_str()
                .read()
            {
                Ok(data) => mk_response(data, StatusCode::SC200),
                Err(_) => mk_response("", StatusCode::SC404),
            },
        )
    };
    route::get("/files").set_response(path().flat_map(read))
}
fn post_file() -> impl Endpoint<Output=UnitT> {
    let response = |(file, body): (String, Option<RequestBody>)| {
        lift(
            match format!("/tmp/data/codecrafters.io/http-server-tester/{}", file)
                .as_str()
                .write(body.unwrap().0)
            {
                Ok(_) => mk_response("", StatusCode::SC201),
                Err(_) => mk_response("", StatusCode::SC404),
            },
        )
    };
    route::post("/files").set_response(path().and(req_body()).flat_map(response))
}
```

For parsing HTTP request byte streams, I use an excellent library, [Nom](https://github.com/rust-bakery/nom), a Rust
parser combinator framework.
It provides a great example of how combinators are built in Rust. Inspired by this approach, I created my own HTTP
combinators.

## Implimentation

There is a trait `Endpoint` that has one abstract function.
`fn handle(&self, r: State) -> Result<(State, Self::Output)>;`
All combinators have to implement it;

I will explain the main idea on the `Map` combinator

```rust
pub trait Endpoint {
    type Output: Debug + Clone;

    fn handle(&self, r: State) -> Result<(State, Self::Output)>;

    fn map<F, O2>(self, f: F) -> Map<Self, F>
    where
        F: Fn(Self::Output) -> O2,
        Self: Sized,
    {
        Map { g: self, f }
    }
}

// Implementation 

pub struct Map<G, F> {
    g: G,
    f: F,
}

impl<G, F, O2> Endpoint for Map<G, F>
where
    O2: Debug + Clone,
    G: Endpoint,
    F: Fn(G::Output) -> O2,
{
    type Output = O2;
    fn handle(&self, r: State) -> Result<(State, Self::Output)> {
        let (state, o) = self.g.handle(r)?;
        Ok((state, (self.f)(o)))
    }
}
//This an example how map is used in a path combinator which returns path of the request 

pub fn path() -> impl Endpoint<Output=String> {
    request().map(|v| v.get_path())
}
```

A few words about `handle`.
It functions like a State monad: a function that takes a `State` and returns a `State` along with the function's `result`.
The `State` type has two variants: `Incomplete`, which contains only an HTTP Request, and `Complete`, which includes both a
Request and a Response.



```rust
#[derive(Debug, Clone)]
pub struct Incomplete(RequestRef);
#[derive(Debug, Clone)]
pub struct Complete(pub RequestRef, pub ResponseRef);
#[derive(Debug, Clone)]
pub enum State {
    Incomplete(Incomplete),
    Complete(Complete),
}
```


`RequestRef` acts as a Reader, providing read-only access to request data for use in combinators.

`ResponseRef` acts as a State, allowing combinators to modify it.

This outlines the basic concept.

The implementation covers only what was required by the challenge.

I’m eager to hear from the Rust community: Is this a sound approach to functional programming in Rust? Are there better
patterns or practices I should consider? Your insights would be invaluable as I continue to grow as a Rust developer.






