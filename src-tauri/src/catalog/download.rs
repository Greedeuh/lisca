use std::path::PathBuf;

pub async fn download_file<F>(url: &str, dest: &PathBuf, on_progress: &mut F) -> Result<(), String>
where
    F: FnMut(u64, u64),
{
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut resp = resp.bytes_stream();
    let mut file = std::fs::File::create(dest).map_err(|e| format!("Create file: {e}"))?;

    use futures_util::StreamExt;
    use std::io::Write;
    while let Some(chunk) = resp.next().await {
        let chunk = chunk.map_err(|e| format!("Read chunk: {e}"))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Write file: {e}"))?;
        downloaded += chunk.len() as u64;
        on_progress(downloaded, total);
    }

    file.flush().map_err(|e| format!("Flush file: {e}"))?;
    Ok(())
}
