use actix_web::{web, App, HttpRequest, HttpServer, Responder};
use nanoid::nanoid;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
struct AppState {
    url_map: HashMap<String, String>,
}



#[derive(Debug, serde::Deserialize,serde::Serialize)]
struct ShortenRequest {
    url: String,
}


async fn shorten_url(req: web::Json<ShortenRequest>, data: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
    let original_url = req.into_inner();
    let short_id = nanoid!(6);
    let short_url = format!("/{}", &short_id);
    // let data = Arc::new(Mutex::new(AppState { url_map: HashMap::new() }));
    // Store the mapping in-memory (replace this with a database in a real application)
//  data.url_map.insert(short_id, original_url.clone());
{
    let mut data = data.lock().unwrap();
    data.url_map.insert(short_id, original_url.url);
}

    format!("Shortened URL: http://localhost:8080{}", short_url)
}




async fn redirect(req: HttpRequest, data: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
    let short_id = req.match_info().get("short_id").unwrap();
    let data = data.lock().unwrap();
    if let Some(original_url) = data.url_map.get(short_id) {
        actix_web::HttpResponse::TemporaryRedirect()
            .header("location", original_url.to_string())
            .finish()
    } else {
        actix_web::HttpResponse::NotFound().body("URL not found")
    }
}
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let state = web::Data::new(AppState {
        url_map: HashMap::new(),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(web::resource("/shorten").route(web::post().to(shorten_url)))
            .service(web::resource("/{short_id}").route(web::get().to(redirect)))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}



#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    #[actix_rt::test]
    async fn test_shorten_url() {
        let data = web::Data::new(Arc::new(Mutex::new(AppState { url_map: HashMap::new() })));
        let data_clone = data.clone();
        let mut app = test::init_service(App::new().service(web::resource("/shorten").route(web::post().to(shorten_url)))).await;

        let payload = ShortenRequest {
            url: "http://example.com".to_owned(),
        };

        let req = test::TestRequest::post().uri("/shorten").set_json(&payload).to_request();
        let response = test::call_service(&mut app, req).await;

        // assert!(response.status().is_success());
        println!("Response Status: {:?}", response.status());

        let body = test::read_body(response).await;
        let body_vec = body.to_vec();
        let body_str = String::from_utf8(body_vec.clone()).unwrap();
        println!("Actual Response Body: {:?}", body_str);
        assert!(body_str.contains("Shortened URL"));

        // Clear the state after the test
        let mut state = data_clone.lock().unwrap();;
        state.url_map.clear();
    }

    #[actix_rt::test]
    async fn test_redirect_to_original_url() {
        let data = web::Data::new(Arc::new(Mutex::new(AppState { url_map: HashMap::new() })));
        let mut app = test::init_service(
            App::new()
                .app_data(data.clone())
                .service(web::resource("/shorten").route(web::post().to(shorten_url)))
                .service(web::resource("/{short_id}").route(web::get().to(redirect))),
        )
        .await;
    
        // Shorten a URL and get the short URL
        let payload = ShortenRequest {
            url: "http://example.com".to_owned(),
        };
        let req = test::TestRequest::post().uri("/shorten").set_json(&payload).to_request();
        let response = test::call_service(&mut app, req).await;
        let body = test::read_body(response).await;
        let body_vec = body.to_vec();
        let body_str = String::from_utf8(body_vec.clone()).unwrap();
        let short_url = body_str.replace("Shortened URL: ", "");
    
        // Now test the redirect
        let req = test::TestRequest::get().uri(&short_url).to_request();
        let response = test::call_service(&mut app, req).await;
    
        assert!(response.status().is_redirection());
        assert_eq!(response.headers().get("location").unwrap(), "http://example.com");
    
        // Clear the state after the test
        let mut state = data.lock().unwrap();
        state.url_map.clear();
    }
}