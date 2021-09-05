use tracing::instrument;

use tera::{Tera, Context};
use std::sync::Mutex;
use crate::network::Client;
use actix_web::{web, HttpRequest, HttpResponse, HttpServer, App};
use actix_web::dev::Server;
use actix_web::web::Data;
use rustls::ServerConfig;

#[allow(clippy::async_yields_async)]
#[instrument]
async fn index(
    template: web::Data<Tera>,
    client: web::Data<Mutex<Client>>,
    _: HttpRequest
) -> HttpResponse {
    let multiaddresses = client.lock().unwrap().get_listen_address().await.unwrap();
    let mut ctx = Context::new();
    ctx.insert("multiaddresses", &multiaddresses);
    ctx.insert("bytes_send", &999);
    ctx.insert("bytes_received", &888);
    let s = template.render("node_interface.html", &ctx).unwrap();
    HttpResponse::Ok().content_type("text/html").body(s)
}

pub async fn make_server(client: Client, config: Option<ServerConfig>, web_http_port: usize, web_https_port: usize) -> Result<Server, std::io::Error> {
    println!("Starting server");
    let mut tera = Tera::default();
    tera.add_raw_template("node_interface.html", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/node_interface.html"))).unwrap();
    let server = HttpServer::new(move || {
        App::new()
            .app_data(Data::new(tera.clone()))
            .app_data(Data::new(Mutex::new(client.clone())))
            .wrap(tracing_actix_web::TracingLogger::default())
            .service(web::resource("/").route(web::get().to(index)))
    });
    let http_address = format!("0.0.0.0:{}", web_http_port);
    let https_address = format!("0.0.0.0:{}", web_https_port);
    let server = match config {
        None => server.bind(http_address)?,
        Some(d) => {
            let server = server.bind_rustls(https_address, d)?;
            server.bind(http_address)?
        }
    };
    let listen_address = server.addrs();
    println!("Web Service is listening on {:?}", listen_address);
    Ok(server.run())
}