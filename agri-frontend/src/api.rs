use gloo_net::http::Request;
use serde::{de::DeserializeOwned, Serialize};

const API_BASE: &str = "/api/v1";

pub async fn get<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let resp = Request::get(&format!("{}{}", API_BASE, path))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.ok() {
        resp.json::<T>().await.map_err(|e| e.to_string())
    } else {
        Err(format!("HTTP {}", resp.status()))
    }
}

#[allow(dead_code)]
pub async fn post<T: Serialize, R: DeserializeOwned>(path: &str, body: &T) -> Result<R, String> {
    let resp = Request::post(&format!("{}{}", API_BASE, path))
        .json(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.ok() {
        resp.json::<R>().await.map_err(|e| e.to_string())
    } else {
        Err(format!("HTTP {}", resp.status()))
    }
}

#[allow(dead_code)]
pub async fn put<T: Serialize, R: DeserializeOwned>(path: &str, body: &T) -> Result<R, String> {
    let resp = Request::put(&format!("{}{}", API_BASE, path))
        .json(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.ok() {
        resp.json::<R>().await.map_err(|e| e.to_string())
    } else {
        Err(format!("HTTP {}", resp.status()))
    }
}

#[allow(dead_code)]
pub async fn delete(path: &str) -> Result<(), String> {
    let resp = Request::delete(&format!("{}{}", API_BASE, path))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.ok() {
        Ok(())
    } else {
        Err(format!("HTTP {}", resp.status()))
    }
}
