use async_trait::async_trait;
use log::info;
use pingora_core::server::configuration::Opt;
use pingora_core::server::Server;
use pingora_core::upstreams::peer::HttpPeer;
use pingora_core::Result;
use pingora_http::ResponseHeader;
use pingora_proxy::{ProxyHttp,Session};

fn extract_branch_id(path: &str) -> Option<String> {
     // Memisahkan path menjadi bagian-bagian yang terpisah berdasarkan karakter '/'
     let parts: Vec<&str> = path.split('/').collect();

     // Memeriksa apakah path memiliki cukup banyak bagian dan format yang sesuai
     println!("Parts {:?}", parts);
     if parts.len() == 4 && parts[1] == "branch" {
         // Mengembalikan nilai branchID yang ditemukan
         Some(parts[2].to_string())
     } else {
         None
     }
}
fn check_login(req: &pingora_http::RequestHeader)->bool{
    return req.headers.get("Authorization").map(|v| v.as_bytes())==Some(b"password");
}
pub struct MyGateway{}

#[async_trait]
impl ProxyHttp for MyGateway{
    type CTX = ();
    fn new_ctx(&self) -> Self::CTX {}

    async fn request_filter(&self, session:&mut Session, _ctx: &mut Self::CTX)-> Result<bool>{
        if session.req_header().uri.path().starts_with("/login") && !check_login(session.req_header()){
            let _ = session.respond_error(403).await;
            return Ok(true)
        }
        let token = session.req_header().headers.get("Authorization");
        if token != None{
            match  token.unwrap().to_str() {
                Ok(_token_str)=>{
                    // println!("Authorization: {:?}", token_str.replace("Bearer ", ""));
                }
                Err(err)=>{
                    println!("Error read token {}", err);
                }
            }
        }
        Ok(false)
    }
    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()>
    where
        Self::CTX: Send + Sync,
    {
        // replace existing header if any
        upstream_response
            .insert_header("Server", "MyGateway")
            .unwrap();
        // because we don't support h3
        upstream_response.remove_header("alt-svc");

        Ok(())
    }

    async fn upstream_peer(&self, session:&mut Session, _ctx: &mut Self::CTX)->Result<Box<HttpPeer>>{
        let addr = match session.req_header().uri.path() {
            "/login" => ("localhost", 8282),
            path => {
                // Memeriksa apakah path sesuai dengan pola "/branch/:branchID/users"
                if let Some(branch_id) = extract_branch_id(path) {
                    // Lakukan sesuatu dengan branch_id, misalnya:
                    println!("Branch ID: {}", branch_id);
                    let _ = session.respond_error(404).await;
                    // return Ok(Box::new(HttpPeer::new(("localhost",8282), false, "localhost".to_string())))
                    // Lakukan sesuatu berdasarkan nilai branch_id yang ditemukan
                }
    
                ("127.0.0.1", 8181)
            }
        };
        println!("Connectin to {:?}", addr);
        let peer = Box::new(HttpPeer::new(addr, false, "localhost".to_string()));
        Ok(peer)
    }
    async fn logging(
        &self,
        session: &mut Session,
        _e: Option<&pingora_core::Error>,
        ctx: &mut Self::CTX,
    ) {
        let response_code = session
            .response_written()
            .map_or(0, |resp| resp.status.as_u16());
        info!(
            "{} response code: {response_code}",
            self.request_summary(session, ctx)
        );
        println!("response code: {response_code}");
    }
}
fn main() {
    env_logger::init();

    // read command line arguments
    let opt = Opt::default();
    let mut my_server = Server::new(Some(opt)).unwrap();
    my_server.bootstrap();

    let mut my_proxy = pingora_proxy::http_proxy_service(
        &my_server.configuration,
        MyGateway {
            
        },
    );
    my_proxy.add_tcp("0.0.0.0:6191");
    my_server.add_service(my_proxy);

    my_server.run_forever();
}
