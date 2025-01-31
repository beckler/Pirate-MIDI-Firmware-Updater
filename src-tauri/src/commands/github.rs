use log::{error, info, trace};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::env::{self, temp_dir};
use std::fs::File;
use std::io::{copy, Cursor};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::device::{ConnectedDevice, ConnectedDeviceType};
use crate::error::{Error, Result};
use crate::github::Release;
use crate::{GITHUB_API_URL, GITHUB_BRIDGE_REPO, GITHUB_CLICK_REPO, GITHUB_ORG, GITHUB_ULOOP_REPO};

#[derive(Serialize, Deserialize)]
struct Query {
    per_page: u32,
    page: u32,
}

fn build_headers() -> HeaderMap {
    // create some headers for our fetching
    let mut headers = HeaderMap::new();

    // add the user-agent header required by github
    headers.insert(USER_AGENT, HeaderValue::from_static("reqwest"));

    // add the authorization header if the enviroment variable GITHUB_TOKEN is defined
    // this is good for developing, as the rate limit for unauthencated requests is 65 requests/hour
    match env::var("GITHUB_TOKEN") {
        Ok(token) => match HeaderValue::from_str(format!("Bearer {}", token).as_str()) {
            Ok(val) => _ = headers.insert(AUTHORIZATION, val),
            Err(err) => error!("GITHUB_TOKEN not set correctly - {err}: {token}"),
        },
        Err(_) => {
            info!("environment variable GITHUB_TOKEN is not set")
        }
    };

    headers
}

async fn get_releases(device: &ConnectedDevice, repo: &str) -> Result<Vec<Release>> {
    // retrieve the releases!
    let url = format!("{}/repos/{}/{}/releases", GITHUB_API_URL, GITHUB_ORG, repo);
    let request = reqwest::Client::new()
        .get(url)
        .headers(build_headers())
        .send();
    match request.await {
        Ok(res) => {
            trace!("success [raw]: {:?}", res);
            match res.status() {
                StatusCode::OK => match res.json::<Vec<Release>>().await {
                    Ok(releases) => {
                        let compatible: Vec<Release> = releases
                            .iter()
                            .filter(|&release| {
                                // find releases compatible with our device
                                release
                                    .assets
                                    .iter()
                                    .any(|asset| asset.is_compatible(device))
                            })
                            .cloned()
                            .collect::<Vec<Release>>();
                        // trace!("compatible releases: {:?}", compatible);
                        Ok(compatible)
                    }
                    Err(err) => err!(Error::Http(err.to_string())),
                },
                StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS => {
                    log::error!("Rate limited from Github - headers: {:?}", res.headers());
                    err!(Error::Http("Github rate limit hit!".to_string()))
                }
                status => {
                    err!(Error::Http(
                        format!("recieved an unsupported http status code: {status}").to_string()
                    ))
                }
            }
        }
        Err(err) => {
            trace!("error [raw]: {:?}", err);
            Err(Error::Http(err.to_string()))
        }
    }
}

#[tauri::command]
/// retrieve all compatable github releases
pub async fn fetch_releases(device: ConnectedDevice) -> Result<Vec<Release>> {
    // perform the fetch
    info!("fetching releases from github...");

    // determine which repo to get
    match &device.device_type {
        ConnectedDeviceType::BridgeBootloader
        | ConnectedDeviceType::RPBootloader
        | ConnectedDeviceType::Unknown => Err(Error::Other(
            "github releases do not exist for this device type".to_string(),
        )),
        ConnectedDeviceType::Bridge4 | ConnectedDeviceType::Bridge6 => {
            get_releases(&device, GITHUB_BRIDGE_REPO).await
        }
        ConnectedDeviceType::Click => get_releases(&device, GITHUB_CLICK_REPO).await,
        ConnectedDeviceType::ULoop => get_releases(&device, GITHUB_ULOOP_REPO).await,
    }
}

/// retrieve specific binary asset and save to the filesystem
pub async fn fetch_compatable_asset(device: &ConnectedDevice, release: Release) -> Result<PathBuf> {
    match release.assets.iter().find(|&a| a.is_compatible(device)) {
        Some(asset) => {
            // download the binary
            info!("fetching asset from github: {}", asset.browser_download_url);
            let request = reqwest::Client::new()
                .get(asset.browser_download_url.clone())
                .headers(build_headers())
                .send();

            // TODO: clean up this fuckin mess
            match request.await {
                Ok(response) => match response.bytes().await {
                    Ok(payload) => {
                        // create timestamp
                        let time = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis();
                        // create temp file
                        let temp_file_path = temp_dir().join(format!("{time}-{}", asset.name));
                        info!("downloading file to: {}", temp_file_path.display());
                        // create temp file
                        match File::create(&temp_file_path) {
                            Ok(mut file) => {
                                let mut content = Cursor::new(payload);
                                match copy(&mut content, &mut file) {
                                    Ok(written) => {
                                        info!(
                                            "successfully downloaded - total bytes written: {}",
                                            written
                                        );
                                        Ok(temp_file_path)
                                    }
                                    Err(err) => err!(Error::IO(err.to_string())),
                                }
                            }
                            Err(err) => err!(Error::IO(err.to_string())),
                        }
                    }
                    Err(err) => err!(Error::Http(err.to_string())),
                },
                Err(err) => err!(Error::Http(err.to_string())),
            }
        }
        None => err!(Error::Http(
            "unable to find compatible asset from release!".to_string()
        )),
    }
}
