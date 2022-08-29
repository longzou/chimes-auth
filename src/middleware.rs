use std::cell::RefCell;
use std::rc::Rc;
use std::task::{Context, Poll};
use actix_web::{Error, error, HttpResponse, web, HttpMessage};
use actix_web::body::{MessageBody, EitherBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::{HeaderValue};
use futures_core::future::{LocalBoxFuture};
use serde::de::DeserializeOwned;

#[cfg(target_feature="session")]
use actix_session::{Session, SessionExt, {storage::SessionStore}};


use crate::{ChimesAuthUser, ApiResult};
use crate::ChimesAuthService;

// The custom ChimesAuthorization for auth
pub struct ChimesAuthorization<T, P> 
where
    T: Clone + Sized + ChimesAuthUser<T> + DeserializeOwned,
    P: ChimesAuthService<T>
{
    #[allow(unused)]
    auth_info: Option<T>,
    auth_service: Rc<P>,
    allow_urls: Rc<Vec<String>>,
    header_key: Option<String>,
    nojwt_header_key: Option<String>,
    #[cfg(target_feature="session")]
    session_key: Option<String>,
}

impl <T, P> ChimesAuthorization<T, P> 
where
    T: Clone + Sized + ChimesAuthUser<T> + DeserializeOwned,
    P: ChimesAuthService<T>
{
    pub fn new(auth_service: P) -> Self {
        Self{
            auth_info: None,
            auth_service: Rc::new(auth_service),
            allow_urls: Rc::new(vec![]),
            header_key: None,
            nojwt_header_key: None,
            #[cfg(target_feature="session")]
            session_key: None,
        }
    }

    pub fn allow(mut self, url: &String) -> Self {
        Rc::get_mut(&mut self.allow_urls)
                .unwrap()
                .push(url.to_string());
        
        self
    }

    pub fn header_key(mut self, new_key: &String) -> Self {
        self.header_key = Some(new_key.to_string());
        self
    }

    pub fn nojwt_header_key(mut self, new_key: &String) -> Self {
        self.nojwt_header_key = Some(new_key.to_string());
        self
    }    

    #[cfg(target_feature="session")]
    pub fn session_key(mut self, new_key: &String) -> Self {
        self.session_key = Some(new_key.to_string());
        self
    }

}

impl<S, B, T, P> Transform<S, ServiceRequest> for ChimesAuthorization<T, P>
    where
        S: Service<ServiceRequest, Response=ServiceResponse<B>, Error=Error> + 'static,
        S::Future: 'static,
        B: MessageBody + 'static,
        T: Clone + Sized + ChimesAuthUser<T> + DeserializeOwned + 'static,
        P: Sized + ChimesAuthService<T>  + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = ChimesAuthenticationMiddleware<S, T, P>;
    type Future = actix_utils::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        actix_utils::future::ok(ChimesAuthenticationMiddleware {
            auth_info: None,
            service: Rc::new(RefCell::new(service)),
            auth_service: self.auth_service.clone(),
            allow_urls: self.allow_urls.clone(),
            header_key: self.header_key.clone(),
            nojwt_header_key: self.nojwt_header_key.clone(),
            #[cfg(target_feature="session")]
            session_key: self.session_key.clone()
        })
    }
}

pub struct ChimesAuthenticationMiddleware<S, T, P> {
    #[allow(unused)]
    auth_info: Option<T>,
    service: Rc<RefCell<S>>,
    auth_service: Rc<P>,
    allow_urls: Rc<Vec<String>>,
    header_key: Option<String>,
    nojwt_header_key: Option<String>,
    #[cfg(target_feature="session")]
    session_key: Option<String>,
}

impl<S, T, P, B> Service<ServiceRequest> for ChimesAuthenticationMiddleware<S, T, P>
    where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
        S::Future: 'static,
        B: MessageBody + 'static,
        T: Clone + Sized + ChimesAuthUser<T> + DeserializeOwned + 'static,
        P: Sized + ChimesAuthService<T>  + 'static,
{
    // type Response = ServiceResponse<B>;
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    // type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>>>>;
    type Future = LocalBoxFuture<'static, Result<ServiceResponse<EitherBody<B>>, Error>>;

    fn poll_ready(self: &ChimesAuthenticationMiddleware<S, T, P>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
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
        let nojwt_header_key = self.nojwt_header_key.clone();

        Box::pin(async move {
            let value = HeaderValue::from_str("").unwrap();
            let token = req.headers().get(header_key.as_str()).unwrap_or(&value);
            let req_method = req.method().to_string();
            
            if passed_url {
                Ok(service.call(req).await?.map_into_left_body())
            } else {
                #[cfg(not(target_feature= "session"))]
                let ust = if nojwt_header_key.is_some() {
                    let nojwt_token = req.headers().get(nojwt_header_key.unwrap().as_str()).unwrap_or(&value);
                    match nojwt_token.to_str() {
                        Ok(st) => {
                            let us = auth.nojwt_authenticate(&st.to_string()).await;
                            us
                        }
                        Err(_) => {
                            None
                        }
                    }
                } else {
                    match token.to_str() {
                        Ok(st) => {
                            let us = auth.authenticate(&st.to_string()).await;
                            us
                        }
                        Err(_) => {
                            None
                        }
                    }
                };

                #[cfg(target_feature= "session")]
                let ust = auth_user;

                let permitted = auth.permit(&ust, &req_method, &url_pattern).await;

                if permitted.is_some() {
                    if ust.is_some() {
                        req.extensions_mut().insert(ust.unwrap().clone());
                    }
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                } else {
                    if ust.is_none() {
                        
                        let err = actix_web::error::ErrorUnauthorized("Not-Authorized");

                        let errresp = req.error_response(err);
                        let wbj: web::Json<ApiResult<String>> = web::Json(ApiResult::error(401, &"Not-Authorized".to_string()));
                        let hrp = HttpResponse::Unauthorized().json(wbj).map_into_boxed_body();
                        
                        let m = ServiceResponse::new(
                            errresp.request().clone(),
                            hrp,
                        );
                        Ok(m.map_into_right_body())
                    } else {
                        let err = actix_web::error::ErrorForbidden("Forbidden");

                        let errresp = req.error_response(err);
                        let wbj: web::Json<ApiResult<String>> = web::Json(ApiResult::error(403, &"Forbidden".to_string()));
                        let hrp = HttpResponse::Forbidden().json(wbj).map_into_boxed_body();
                        
                        let m = ServiceResponse::new(
                            errresp.request().clone(),
                            hrp,
                        );
                        Ok(m.map_into_right_body())
                    }
                }
            }
        })

    }
}
