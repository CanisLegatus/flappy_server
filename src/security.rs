use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Duration, TimeZone, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::{Rng, distr::Alphanumeric};
use serde::{Deserialize, Serialize};
use tower_governor::key_extractor::KeyExtractor;

use crate::{error::JwtError, state::AppState};

pub trait TimeProvider {
    fn now(&self) -> DateTime<chrono::Utc>;
}

pub struct RealTime;

impl TimeProvider for RealTime {
    fn now(&self) -> DateTime<chrono::Utc> {
        Utc::now()
    }
}

pub struct MockTime;

impl TimeProvider for MockTime {
    fn now(&self) -> DateTime<chrono::Utc> {
        chrono::Utc
            .with_ymd_and_hms(2015, 3, 15, 12, 0, 0)
            .single()
            .expect("Can't get time in MockTime")
    }
}

#[derive(Clone)]
pub struct JwtKeyExtractor;

impl KeyExtractor for JwtKeyExtractor {
    type Key = String;

    fn extract<T>(
        &self,
        req: &axum::http::Request<T>,
    ) -> Result<Self::Key, tower_governor::GovernorError> {
        req.headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|token| token.to_string())
            .ok_or(tower_governor::errors::GovernorError::UnableToExtractKey)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    pub sub: String,
    pub exp: usize,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
}

#[derive(Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub validation: Validation,
}

impl JwtConfig {
    pub fn new(secret: String) -> Self {
        let mut validation = Validation::default();
        validation.leeway = 60;
        validation.validate_exp = true;
        validation.validate_nbf = true;

        Self {
            secret,
            validation,
        }
    }
}

pub async fn jwt_middleware(
    req: Request<Body>,
    next: Next,
    state: AppState,
) -> Result<Response, JwtError> {
    let token = req
        .headers()
        .get("Authorization")
        .ok_or(JwtError::MissingAuthHeader)?
        .to_str()
        .map_err(|_| JwtError::InvalidTokenFormat)?
        .strip_prefix("Bearer ")
        .ok_or(JwtError::InvalidTokenFormat)?
        .trim();

    let secret = &state.jwt_config.read().await.secret;
    let validation = &state.jwt_config.read().await.validation;

    let _claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        validation,
    )
    .map_err(JwtError::DecodeError)?
    .claims;

    Ok(next.run(req).await)
}

pub fn generate_jwt(user_id: &str, secret: &str, role: &str, time: &impl TimeProvider) -> Result<String, JwtError> {
    let expiration = time.now() 
        .checked_add_signed(Duration::hours(1))
        .expect("Invalid timestamp! Server is shutdown!")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_owned(),
        exp: expiration,
        role: role.to_owned(),
    };

    encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .map_err(JwtError::_EncodingError)
}

pub async fn validate_user(_username: &str, _password: &str) -> Result<User, String> {
    Ok(User {
        id: "default".into(),
    })
}

pub async fn set_up_security_headers(
    req: axum::http::Request<Body>,
    next: axum::middleware::Next,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    let mut response = next.run(req).await;

    // Sources of content limited only to our domain
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'self'"),
    );

    // Use only HTTPS in a year (will fail in local)
    response.headers_mut().insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    // Restict guessing on content type
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );

    // Restrict usage in iframes on other sites, apps
    response
        .headers_mut()
        .insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));

    // Restict sending info when of parameters when downgrate from https to http
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer-when-downgrade"),
    );
    // Restrict usage of geolocation, camera
    response.headers_mut().insert(
        "Permissions-Policy",
        HeaderValue::from_static("geolocation=(), camera=()"),
    );

    Ok(response)
}

pub fn generate_secret() -> String {
    let mut rng = rand::rng();
    (0..32).map(|_| rng.sample(Alphanumeric) as char).collect()
}

#[cfg(test)]
mod security_tests {
    use axum::{http::{Request, StatusCode}, middleware, Router};
    use jsonwebtoken::Algorithm;
    use axum::routing::method_routing::get;
    use tower::ServiceExt;
    use super::*;
    
    #[tokio::test]
    async fn test_set_up_security_headers(){

        let request = Request::builder()
            .uri("/test")
            .method("GET")
            .header("User-agent", "test-agent")
            .body(Body::empty())
            .expect("Can't create request");
        
        let app = Router::new()
            .route("/test", get(|| async {"Hello"}))
            .layer(middleware::from_fn(set_up_security_headers));

        let res: Response<Body> = app.oneshot(request).await.expect("Can't get response!");        
        let headers = res.headers();

        assert_eq!(res.status(), StatusCode::OK);
        assert!(headers.contains_key("Content-Security-Policy"));
        assert!(headers.contains_key("Strict-Transport-Security"));
        assert!(headers.contains_key("X-Content-Type-Options"));
        assert!(headers.contains_key("X-Frame-Options"));
        assert!(headers.contains_key("Referrer-Policy"));
        assert!(headers.contains_key("Permissions-Policy"));
        
        assert_eq!(headers.get("Content-Security-Policy").unwrap(),"default-src 'self'");
        assert_eq!(headers.get("Strict-Transport-Security").unwrap(),"max-age=31536000; includeSubDomains");
        assert_eq!(headers.get("X-Content-Type-Options").unwrap(),"nosniff");        
        assert_eq!(headers.get("X-Frame-Options").unwrap(),"DENY");
        assert_eq!(headers.get("Referrer-Policy").unwrap(),"no-referrer-when-downgrade");
        assert_eq!(headers.get("Permissions-Policy").unwrap(),"geolocation=(), camera=()");

    }

    #[tokio::test]
    async fn test_jwt_generate() {
        let test_user_id = "test_user";
        let test_secret = "serious_secret";
        let test_role = "great_leader";
        let mut no_time_validation = Validation::new(Algorithm::HS256);
        no_time_validation.validate_exp = false;

        let token = generate_jwt(test_user_id, test_secret, test_role, &MockTime).expect("Can't genereate jwt it test!");
        
        //Creating decode that will fail because of old data
        let cursed_decode = decode::<Claims>(&token,
            &DecodingKey::from_secret(test_secret.as_bytes()), &Validation::new(Algorithm::HS256));
        
        //Creating token_data to get Claims
        let token_data = decode::<Claims>(&token,
            &DecodingKey::from_secret(test_secret.as_bytes()), &no_time_validation).expect("Can't decode test jwt!"); 
        
        let claims = token_data.claims;
        
        assert!(cursed_decode.is_err());
        assert_eq!(&claims.sub, test_user_id);
        assert_eq!(&claims.role, test_role);
        assert_eq!(&claims.exp, &((MockTime.now() + Duration::hours(1)).timestamp() as usize));

    }
}






