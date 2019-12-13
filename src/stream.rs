use crate::opt::Quality;
use failure::{bail, format_err, Error, ResultExt};
use futures::AsyncReadExt;
use http_client::{native::NativeClient, Body, HttpClient};

pub async fn get_master_url(url: &str) -> Result<String, Error> {
    let uri = url.parse::<http::Uri>().context("Failed to build URI")?;
    let request = http::Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();

    let client = NativeClient::default();
    let resp = client.send(request).await?;

    let mut body = resp.into_body();
    let mut body_text = String::new();
    body.read_to_string(&mut body_text)
        .await
        .context("Failed to read response body text")?;

    if !&body_text[..].starts_with("https") {
        bail!("Stream not available yet");
    }

    Ok(body_text)
}

pub async fn get_master_m3u8(url: &str) -> Result<String, Error> {
    let uri = url.parse::<http::Uri>().context("Failed to build URI")?;
    let request = http::Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();

    let client = NativeClient::default();
    let resp = client.send(request).await?;

    let mut body = resp.into_body();
    let mut body_text = String::new();
    body.read_to_string(&mut body_text)
        .await
        .context("Failed to read response body text")?;

    if body_text[..].starts_with("#EXTM3U") {
        return Ok(body_text);
    }

    bail!("Failed to get master m3u8");
}

pub fn get_quality_url(
    master_url: &str,
    master_m3u8: &str,
    quality: Quality,
) -> Result<String, Error> {
    let quality_str: &str = quality.clone().into();
    let quality_check = format!("x{}", quality_str);

    let mut quality_idx = None;
    for (idx, line) in master_m3u8.lines().enumerate() {
        if (quality == Quality::_720p60 && line.contains("FRAME-RATE"))
            || (quality != Quality::_720p60 && line.contains(&quality_check))
        {
            quality_idx = Some(idx + 1);
        }
    }

    if let Some(idx) = quality_idx {
        let quality_line = master_m3u8
            .lines()
            .nth(idx)
            .ok_or_else(|| format_err!("No stream found matching quality specified"))?;

        let master_url_parts = master_url.rsplitn(2, '/').collect::<Vec<&str>>();
        if master_url_parts.len() == 2 {
            let quality_url = format!("{}/{}", master_url_parts[1], quality_line);

            return Ok(quality_url);
        }
    }

    bail!("No stream found matching quality specified");
}
