use tracing::instrument;

use tera::{Tera, Context};
use std::sync::Mutex;
use crate::network::Client;
use actix_web::{web, HttpRequest, HttpResponse, HttpServer, App};
use actix_web::dev::Server;
use actix_web::web::Data;

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

pub async fn make_server(client: Client) -> Result<Server, std::io::Error> {
    println!("Starting server");
    let mut tera = Tera::default();
    tera.add_raw_template("node_interface.html", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/node_interface.html"))).unwrap();
    Ok(HttpServer::new(move || {
        App::new()
            .app_data(Data::new(tera.clone()))
            .app_data(Data::new(Mutex::new(client.clone())))
            .wrap(tracing_actix_web::TracingLogger::default())
            .service(web::resource("/").route(web::get().to(index)))
    })
        .bind("0.0.0.0:8080")?
        .run())
}