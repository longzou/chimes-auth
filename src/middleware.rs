use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use actix_web::{Error, error};
use actix_web::body::{MessageBody, EitherBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::{HeaderValue};
use futures::Future;
use futures::future::{ok, Ready};
use serde::de::DeserializeOwned;

#[cfg(target_feature="session")]
use actix_session::{Session, SessionExt, {storage::SessionStore}};


use crate::ChimesAuthUser;
use crate::ChimesAuthService;

// The custom ChimesAuthorization for auth
pub struct ChimesAuthorization<T> 
where
    T: Sized + ChimesAuthService<T> + ChimesAuthUser<T> + DeserializeOwned
{
    auth_service: Rc<T>,
    allow_urls: Rc<Vec<String>>,
    header_key: Option<String>,
    #[cfg(target_feature="session")]
    session_key: Option<String>,
}

impl <T> ChimesAuthorization<T> 
where
    T: Sized + ChimesAuthService<T> + ChimesAuthUser<T> + DeserializeOwned,
{
    pub fn new(auth_service: T) -> Self {
        Self{
            auth_service: Rc::new(auth_service),
            allow_urls: Rc::new(vec![]),
            header_key: None,
            #[cfg(target_feature="session")]
            session_key: None,
        }
    }

    pub fn allow(&mut self, url: &String) -> &mut Self {
        Rc::get_mut(&mut self.allow_urls)
                .unwrap()
                .push(url.to_string());
        
        self
    }

    pub fn header_key(&mut self, new_key: &String) -> &mut Self {
        self.header_key = Some(new_key.to_string());
        self
    }

    #[cfg(target_feature="session")]
    pub fn session_key(&mut self, new_key: &String) -> &mut Self {
        self.session_key = Some(new_key.to_string());
        self
    }
}

impl<S, B, T> Transform<S, ServiceRequest> for ChimesAuthorization<T>
    where
        S: Service<ServiceRequest, Response=ServiceResponse<EitherBody<B>>, Error=Error> + 'static,
        S::Future: 'static,
        B: MessageBody + 'static,
        T: Sized + ChimesAuthService<T> + ChimesAuthUser<T> + DeserializeOwned + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = ChimesAuthenticationMiddleware<S, T>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(ChimesAuthenticationMiddleware {
            service: Rc::new(RefCell::new(service)),
            auth_service: self.auth_service.clone(),
            allow_urls: self.allow_urls.clone(),
            header_key: self.header_key.clone(),
            #[cfg(target_feature="session")]
            session_key: self.session_key.clone()
        })
    }
}

pub struct ChimesAuthenticationMiddleware<S, T> {
    service: Rc<RefCell<S>>,
    auth_service: Rc<T>,
    allow_urls: Rc<Vec<String>>,
    header_key: Option<String>,
    #[cfg(target_feature="session")]
    session_key: Option<String>,    
}

impl<S, T, B> Service<ServiceRequest> for ChimesAuthenticationMiddleware<S, T>
    where
        S: Service<ServiceRequest, Response = ServiceResponse<EitherBody<B>>, Error = Error> + 'static,
        S::Future: 'static,
        B: MessageBody + 'static,
        T: Sized + ChimesAuthService<T> + ChimesAuthUser<T> + DeserializeOwned + 'static,
{
    // type Response = ServiceResponse<B>;
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>>>>;
    // type Future = LocalBoxFuture<'static, Result<ServiceResponse<EitherBody<B>>, Error>>;

    fn poll_ready(self: &ChimesAuthenticationMiddleware<S, T>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let auth = self.auth_service.clone();
        let url_pattern = req.match_pattern().unwrap_or_default();
        let passed_url = self.allow_urls.contains(&url_pattern);

        #[cfg(target_feature="session")]
        let session = req.get_session();
        #[cfg(target_feature="session")]
        let default_session_key = "chimes-logged-user".to_string();
        #[cfg(target_feature="session")]
        let auth_user = session.get::<T>(self.session_key.clone().unwrap_or(default_session_key).as_str()).unwrap();
        

        let header_key = self.header_key.clone().unwrap_or("Authentication".to_string());

        Box::pin(async move {
            let value = HeaderValue::from_str("").unwrap();
            let token = req.headers().get(header_key.as_str()).unwrap_or(&value);
            let req_method = req.method().to_string();
            
            if passed_url {
                Ok(service.call(req).await?)
            } else {
                #[cfg(not(target_feature= "session"))]
                let ust = match token.to_str() {
                    Ok(st) => {
                        auth.authenticate(&st.to_string())
                    }
                    Err(_) => {
                        None
                    }
                };

                #[cfg(target_feature= "session")]
                let ust = auth_user;

                if auth.permit(&ust, &req_method, &url_pattern) {
                    Ok(service.call(req).await?)
                } else {
                    let res = req.error_response(error::ErrorUnauthorized("err"));
                    Ok(res.map_into_right_body())
                }
            }
        })

    }
}