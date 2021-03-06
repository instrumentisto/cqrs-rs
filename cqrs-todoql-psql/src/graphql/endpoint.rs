use iron::{
    self,
    headers::ContentType,
    mime::{Mime, SubLevel, TopLevel},
};
use juniper::http::graphiql::graphiql_source;
use juniper_iron::GraphQLHandler;
use mount::Mount;
use std::sync::Arc;

use super::{
    schema::{Mutations, Query},
    Context, InnerContext,
};

pub fn create_chain(context: InnerContext) -> iron::Chain {
    let context_arc = Arc::new(context);

    let context_factory = move |_: &mut iron::Request| {
        Ok(Context {
            inner: Arc::clone(&context_arc),
        })
    };

    let mut mount = Mount::new();

    let graphql_endpoint = GraphQLHandler::new(context_factory, Query, Mutations);

    mount.mount("/graphql", graphql_endpoint);
    mount.mount("/graphiql", |req: &mut iron::Request| {
        let url = req.url.as_ref().join("/graphql").unwrap();
        let graphiql_text = graphiql_source(url.as_ref());

        let json_header = ContentType(Mime(TopLevel::Text, SubLevel::Html, Vec::new()));

        let mut res = iron::Response::new();
        res.body = Some(Box::new(graphiql_text));
        res.headers.set(json_header);
        Ok(res)
    });

    iron::Chain::new(mount)
}
