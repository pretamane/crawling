use headless_chrome::{Browser, LaunchOptions};
use std::sync::Arc;
use anyhow::Result;

pub async fn crawl_with_chrome(url: &str) -> Result<String> {
    let browser = Browser::new(LaunchOptions {
        headless: true,
        ..Default::default()
    })?;

    let tab = browser.new_tab()?;
    tab.navigate_to(url)?;
    tab.wait_until_navigated()?;
    
    // Wait for some JS to load if needed, or just grab content
    let content = tab.get_content()?;
    Ok(content)
}

pub async fn crawl_fast(url: &str) -> Result<String> {
    let resp = reqwest::get(url).await?.text().await?;
    Ok(resp)
}
