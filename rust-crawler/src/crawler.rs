use headless_chrome::{Browser, LaunchOptions};
use anyhow::Result;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::time::Duration;
use tokio::time::sleep;
use once_cell::sync::Lazy;
use regex::Regex;

// Import from new proxy module
use crate::proxy::{PROXY_MANAGER, generate_proxy_auth_extension};

static USER_AGENTS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:124.0) Gecko/20100101 Firefox/124.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:124.0) Gecko/20100101 Firefox/124.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Edge/123.0.0.0 Safari/537.36",
    ]
});

// ============================================================================
// Enhanced Data Structures for Deep Extraction
// ============================================================================

/// Basic search result from SERP
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub title: String,
    pub link: String,
    pub snippet: String,
}

/// Enhanced SERP data with additional extracted elements
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SerpData {
    /// Organic search results
    pub results: Vec<SearchResult>,
    /// "People Also Ask" questions (Google)
    pub people_also_ask: Vec<String>,
    /// Related searches at bottom of page
    pub related_searches: Vec<String>,
    /// Featured snippet if present
    pub featured_snippet: Option<FeaturedSnippet>,
    /// Total results count (if shown)
    pub total_results: Option<String>,
}

/// Featured snippet content
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeaturedSnippet {
    pub content: String,
    pub source_url: Option<String>,
    pub source_title: Option<String>,
}

/// Deep website data extraction
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct WebsiteData {
    // Basic metadata
    pub url: String,
    pub final_url: String,
    pub title: String,
    pub meta_description: Option<String>,
    pub meta_keywords: Option<String>,
    pub meta_author: Option<String>,
    pub meta_date: Option<String>,
    
    // Content extraction
    pub main_text: String,
    // HTML content (for saving to file)
    #[serde(skip)] 
    pub html: String,
    pub word_count: u32,
    pub html_size: u32,
    
    // Structured data (JSON-LD, Schema.org)
    pub schema_org: Vec<serde_json::Value>,
    
    // Open Graph data
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
    pub og_type: Option<String>,
    
    // Contact information
    pub emails: Vec<String>,
    pub phone_numbers: Vec<String>,
    
    // Media
    pub images: Vec<ImageData>,
    
    // Links
    pub outbound_links: Vec<String>,
}

/// Image data with metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageData {
    pub src: String,
    pub alt: Option<String>,
    pub title: Option<String>,
}

/// Complete crawl result with all extracted data
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CrawlResult {
    pub keyword: String,
    pub engine: String,
    pub serp_data: SerpData,
    pub first_result_data: Option<WebsiteData>,
}

#[derive(Debug, Clone, Default)]
pub struct ExtractedContent {
    pub html: String,
    pub text: String,
    pub meta_description: Option<String>,
    pub meta_author: Option<String>,
    pub meta_date: Option<String>,
}

// ============================================================================
// Extraction Helper Functions
// ============================================================================

/// Extract emails from text using regex
pub fn extract_emails(text: &str) -> Vec<String> {
    let email_regex = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap();
    email_regex
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

/// Extract phone numbers from text using regex
pub fn extract_phone_numbers(text: &str) -> Vec<String> {
    let phone_regex = Regex::new(r"[\+]?[(]?[0-9]{1,3}[)]?[-\s\.]?[(]?[0-9]{1,4}[)]?[-\s\.]?[0-9]{1,4}[-\s\.]?[0-9]{1,9}").unwrap();
    phone_regex
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .filter(|p| p.len() >= 7) // Filter out short matches
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

/// Extract Schema.org JSON-LD data from HTML
pub fn extract_schema_org(html: &str) -> Vec<serde_json::Value> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("script[type='application/ld+json']").unwrap();
    
    document
        .select(&selector)
        .filter_map(|el| {
            let json_text = el.text().collect::<String>();
            serde_json::from_str(&json_text).ok()
        })
        .collect()
}

/// Extract Open Graph metadata
pub fn extract_open_graph(document: &Html) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let og_title = document
        .select(&Selector::parse("meta[property='og:title']").unwrap())
        .next()
        .and_then(|el| el.value().attr("content").map(|s| s.to_string()));
    
    let og_description = document
        .select(&Selector::parse("meta[property='og:description']").unwrap())
        .next()
        .and_then(|el| el.value().attr("content").map(|s| s.to_string()));
    
    let og_image = document
        .select(&Selector::parse("meta[property='og:image']").unwrap())
        .next()
        .and_then(|el| el.value().attr("content").map(|s| s.to_string()));
    
    let og_type = document
        .select(&Selector::parse("meta[property='og:type']").unwrap())
        .next()
        .and_then(|el| el.value().attr("content").map(|s| s.to_string()));
    
    (og_title, og_description, og_image, og_type)
}

/// Extract images with metadata
pub fn extract_images(document: &Html, base_url: &str) -> Vec<ImageData> {
    let img_selector = Selector::parse("img").unwrap();
    
    document
        .select(&img_selector)
        .filter_map(|el| {
            let src = el.value().attr("src").or_else(|| el.value().attr("data-src"))?;
            // Skip tiny/tracking pixels
            if src.contains("1x1") || src.contains("pixel") || src.len() < 10 {
                return None;
            }
            Some(ImageData {
                src: if src.starts_with("http") { src.to_string() } else { format!("{}{}", base_url, src) },
                alt: el.value().attr("alt").map(|s| s.to_string()),
                title: el.value().attr("title").map(|s| s.to_string()),
            })
        })
        .take(20) // Limit to first 20 images
        .collect()
}

/// Extract outbound links
pub fn extract_outbound_links(document: &Html, base_domain: &str) -> Vec<String> {
    let link_selector = Selector::parse("a[href]").unwrap();
    
    document
        .select(&link_selector)
        .filter_map(|el| el.value().attr("href").map(|s| s.to_string()))
        .filter(|href| href.starts_with("http") && !href.contains(base_domain))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .take(50) // Limit to 50 links
        .collect()
}


pub async fn search_bing(keyword: &str) -> Result<SerpData> {
    use rand::seq::SliceRandom;
    // Select a random User-Agent
    let user_agent = USER_AGENTS.choose(&mut rand::thread_rng())
        .unwrap_or(&"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36");
    
    println!("Using User-Agent: {}", user_agent);

    // Use anonymous/incognito mode (no profile persistence)
    let mut args = vec![
        std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
        std::ffi::OsStr::new("--no-sandbox"),
        std::ffi::OsStr::new("--disable-dev-shm-usage"),
        std::ffi::OsStr::new("--disable-infobars"),
        std::ffi::OsStr::new("--window-position=0,0"),
        std::ffi::OsStr::new("--ignore-certificate-errors"),
        std::ffi::OsStr::new("--ignore-certificate-errors-spki-list"),
    ];
    let ua_arg = format!("--user-agent={}", user_agent);
    args.push(std::ffi::OsStr::new(&ua_arg));

    // Add proxy if available (using new ProxyManager)
    let proxy_arg: String;
    let ext_arg: String;
    let current_proxy = PROXY_MANAGER.get_next_proxy();
    let _proxy_id = current_proxy.as_ref().map(|p| p.id.clone());
    
    if let Some(ref proxy) = current_proxy {
        println!("🔄 Using proxy: {} (healthy: {}, success_rate: {:.1}%)", 
            proxy.id, 
            proxy.healthy.load(std::sync::atomic::Ordering::Relaxed),
            proxy.success_rate() * 100.0
        );
        proxy_arg = format!("--proxy-server={}", proxy.to_chrome_arg());
        args.push(std::ffi::OsStr::new(&proxy_arg));
        
        // Add auth extension if proxy requires authentication
        if proxy.requires_auth() {
            let ext_path = generate_proxy_auth_extension(
                proxy.username.as_ref().unwrap(),
                proxy.password.as_ref().unwrap()
            );
            ext_arg = format!("--load-extension={}", ext_path);
            args.push(std::ffi::OsStr::new(&ext_arg));
            println!("🔐 Proxy auth extension loaded");
        }
    }

    let browser = Browser::new(LaunchOptions {
        headless: true,
        window_size: Some((1920, 1080)),
        args,
        ..Default::default()
    })?;

    let tab = browser.new_tab()?;

    // Layer 1: Device & Environment Fingerprinting (JS-Level)
    // Inject stealth scripts to run before any other script on the page
    // Inject stealth scripts to run before any other script on the page
    let stealth_script = r#"
        // 1. Remove navigator.webdriver
        Object.defineProperty(navigator, 'webdriver', {
            get: () => undefined,
        });

        // 2. Spoof Hardware Concurrency
        Object.defineProperty(navigator, 'hardwareConcurrency', {
            get: () => 4,
        });

        // 3. Canvas Noise (Perlin-like jitter)
        const originalToDataURL = HTMLCanvasElement.prototype.toDataURL;
        HTMLCanvasElement.prototype.toDataURL = function(...args) {
            if (this.width > 0 && this.height > 0) {
                const context = this.getContext('2d');
                if (context) {
                    const imageData = context.getImageData(0, 0, this.width, this.height);
                    for (let i = 0; i < this.height; i++) {
                        for (let j = 0; j < this.width; j++) {
                            const index = ((i * (this.width * 4)) + (j * 4));
                            // Add subtle noise to alpha channel
                            if (imageData.data[index + 3] > 0) {
                                imageData.data[index + 3] = Math.max(0, Math.min(255, imageData.data[index + 3] + (Math.random() > 0.5 ? 1 : -1)));
                            }
                        }
                    }
                    context.putImageData(imageData, 0, 0);
                }
            }
            return originalToDataURL.apply(this, args);
        };
        
        // 4. WebGL Vendor Spoofing
        const getParameter = WebGLRenderingContext.prototype.getParameter;
        WebGLRenderingContext.prototype.getParameter = function(parameter) {
            // UNMASKED_VENDOR_WEBGL
            if (parameter === 37445) return 'Intel Inc.';
            // UNMASKED_RENDERER_WEBGL
            if (parameter === 37446) return 'Intel Iris OpenGL Engine';
            return getParameter.apply(this, [parameter]);
        };
        
        // 5. Chrome Runtime (Mocking)
        window.chrome = {
            runtime: {},
            loadTimes: function() {},
            csi: function() {},
            app: {}
        };

        // 6. Block WebRTC (prevent IP leaks)
        ['RTCPeerConnection', 'webkitRTCPeerConnection', 'mozRTCPeerConnection', 'msRTCPeerConnection'].forEach(className => {
             if (window[className]) {
                 window[className] = undefined;
             }
        });
    "#;

    // Enable Page domain to use addScriptToEvaluateOnNewDocument
    tab.enable_debugger()?;
    tab.call_method(headless_chrome::protocol::cdp::Page::AddScriptToEvaluateOnNewDocument {
        source: stealth_script.to_string(),
        world_name: None,
        include_command_line_api: None,
        run_immediately: None,
    })?;

    // 1. Navigate to Home
    println!("Navigating to Bing Home...");
    tab.navigate_to("https://www.bing.com/?cc=US")?;
    tab.wait_until_navigated()?;
    
    // 2. Type Query (Layer 3: Typing Speed)
    let search_box = tab.wait_for_element("input[name='q']")?;
    search_box.click()?;
    
    // Clear any existing content (important for fresh search)
    println!("Clearing search box...");
    tab.evaluate(r#"
        const input = document.querySelector('input[name="q"]');
        if (input) { input.value = ''; input.focus(); }
    "#, false)?;
    sleep(Duration::from_millis(200)).await;
    
    println!("Typing query: {}...", keyword);
    for char in keyword.chars() {
        tab.type_str(&char.to_string())?;
        // Random typing delay (80-200ms)
        sleep(Duration::from_millis(80 + (rand::random::<u64>() % 120))).await;
    }
    
    // 3. Submit
    tab.press_key("Enter")?;
    tab.wait_until_navigated()?;
    println!("Search submitted.");
    
    // Layer 3: Behavioral Realism (Human-Like Interaction)
    // Random mouse movements via JS (Bezier-like curves simulated with steps)
    let _ = tab.evaluate(r#"
        function bezier(t, p0, p1, p2, p3) {
            const cX = 3 * (p1.x - p0.x), bX = 3 * (p2.x - p1.x) - cX, aX = p3.x - p0.x - cX - bX;
            const cY = 3 * (p1.y - p0.y), bY = 3 * (p2.y - p1.y) - cY, aY = p3.y - p0.y - cY - bY;
            const x = (aX * Math.pow(t, 3)) + (bX * Math.pow(t, 2)) + (cX * t) + p0.x;
            const y = (aY * Math.pow(t, 3)) + (bY * Math.pow(t, 2)) + (cY * t) + p0.y;
            return {x: x, y: y};
        }

        async function humanMouseMove(startX, startY, endX, endY, steps) {
            // Random control points for Bezier curve
            const p0 = {x: startX, y: startY};
            const p3 = {x: endX, y: endY};
            const p1 = {x: startX + (Math.random() * (endX - startX)), y: startY + (Math.random() * (endY - startY))};
            const p2 = {x: startX + (Math.random() * (endX - startX)), y: startY + (Math.random() * (endY - startY))};

            for (let i = 0; i <= steps; i++) {
                const t = i / steps;
                const pos = bezier(t, p0, p1, p2, p3);
                
                document.dispatchEvent(new MouseEvent('mousemove', {
                    view: window,
                    bubbles: true,
                    cancelable: true,
                    clientX: pos.x,
                    clientY: pos.y
                }));
                // Non-linear timing
                await new Promise(r => setTimeout(r, 10 + Math.random() * 15));
            }
        }
        humanMouseMove(100, 100, 500, 400, 25);
    "#, false)?;
    
    sleep(Duration::from_millis(500)).await;

    // Light scroll simulation (non-blocking, limited scroll)
    let _ = tab.evaluate(r#"
        (function() {
            let scrolled = 0;
            const interval = setInterval(() => {
                window.scrollBy(0, 50 + Math.random() * 50);
                scrolled += 100;
                if (scrolled > 600) {
                    clearInterval(interval);
                    window.scrollBy(0, -200); // Scroll back up slightly
                }
            }, 100 + Math.random() * 100);
        })();
    "#, false)?;  // Non-blocking
    
    // Wait for JavaScript to render results
    println!("Waiting for Bing DOM mutations to complete...");
    sleep(Duration::from_secs(3)).await;  // Simple wait for page to settle
    
    // Improved Bing Selectors (Robust)
    // 1. Check for Challenge first
    let html_content = tab.get_content()?;
    let challenge_patterns = [
        "Prove you're not a robot",
        "humanity",
        "unusual traffic",
        "automated requests",
        "hcaptcha",
        "recaptcha",
        "turnstile",
        "security check",
        "One last step"
    ];
    let is_challenge = challenge_patterns.iter().any(|p| html_content.to_lowercase().contains(&p.to_lowercase()));

    if is_challenge {
         eprintln!("⚠️ CHALLENGE DETECTED: Bing served Challenge/Captcha page via AWS IP");
         let _ = std::fs::write("debug/debug_bing_challenge_detected.html", &html_content);
         if let Ok(screenshot) = tab.capture_screenshot(
            headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
            None, None, true
         ) {
             let _ = std::fs::write("debug/debug_bing_challenge.png", &screenshot);
         }
         return Err(anyhow::anyhow!("Bing Challenge Detected")); // Fail early to trigger retry/proxy rotation if implemented
    }

    // 2. Wait for ANY valid result container
    println!("Waiting for Bing results...");
    let result_wait = tab.wait_for_element_with_custom_timeout("#b_results > li.b_algo, #b_pole, .b_algo", Duration::from_secs(10));
    
    match result_wait {
        Ok(_) => println!("Found results element."),
        Err(e) => {
             println!("Wait for results timed out: {}", e);
             // Dump debug info
             let _ = std::fs::write("debug/debug_bing_no_results.html", &tab.get_content().unwrap_or_default());
        },
    }
    
    // Take screenshot for debugging
    println!("Capturing Bing screenshot...");
    if let Ok(screenshot) = tab.capture_screenshot(
        headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
        None,
        None,
        true
    ) {
        let _ = std::fs::write("debug/debug_bing_screenshot.png", &screenshot);
        println!("Screenshot saved to debug/debug_bing_screenshot.png");
    }

    let html_content = tab.get_content()?;
    println!("Got content. Length: {}", html_content.len());
    let document = Html::parse_document(&html_content);
    
    // Bing Selectors
    let result_selector = Selector::parse("li.b_algo").unwrap();
    let title_selector = Selector::parse("h2 > a").unwrap();
    let snippet_selector = Selector::parse("p").unwrap();

    let mut results = Vec::new();

    for element in document.select(&result_selector) {
        let title = element.select(&title_selector).next().map(|e| e.text().collect::<String>());
        let link = element.select(&title_selector).next().and_then(|e| e.value().attr("href").map(|s| s.to_string()));
        let snippet = element.select(&snippet_selector).next().map(|e| e.text().collect::<String>());

        if let (Some(title), Some(link)) = (title, link) {
            results.push(SearchResult {
                title,
                link,
                snippet: snippet.unwrap_or_default(),
            });
        }
    }
    
    println!("Found {} results.", results.len());

    // Tier 1+ Challenge Detection
    let challenge_patterns = [
        "Prove you're not a robot",
        "Prove your humanity",
        "unusual traffic",
        "automated requests",
        "hcaptcha",
        "recaptcha",
        "blocked",
    ];
    
    let is_challenge = challenge_patterns.iter().any(|p| html_content.to_lowercase().contains(&p.to_lowercase()));
    let is_too_small = html_content.len() < 50_000; // Normal Bing SERP is ~200KB+
    
    if is_challenge {
        eprintln!("⚠️ CHALLENGE DETECTED: Bing served CAPTCHA/challenge page");
        let _ = std::fs::write("debug/debug_bing_challenge.html", &html_content);
    }
    
    if results.is_empty() {
        let failure_reason = if is_challenge {
            "challenge_detected"
        } else if is_too_small {
            "page_too_small"
        } else {
            "no_results_found"
        };
        
        eprintln!("Bing returned 0 results. Reason: {}, HTML len: {}", failure_reason, html_content.len());
        let _ = std::fs::write("debug/debug_bing_tier1.html", &html_content);
        
        // Log failure for metrics
        let log_entry = format!(
            "{{\"timestamp\":\"{}\",\"engine\":\"bing\",\"keyword\":\"{}\",\"reason\":\"{}\",\"html_len\":{}}}\n",
            chrono::Utc::now().to_rfc3339(),
            keyword,
            failure_reason,
            html_content.len()
        );
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/crawl_failures.log")
            .and_then(|mut f| std::io::Write::write_all(&mut f, log_entry.as_bytes()));
    }

    // Extract Related Searches (Bing)
    let related_selector = Selector::parse("li.b_ans ul li a, .b_rs ul li a").unwrap();
    let mut related_searches = Vec::new();
    for element in document.select(&related_selector) {
         if let Some(text) = element.text().next() {
             related_searches.push(text.to_string());
         }
    }
    
    // Extract Total Results
    let count_selector = Selector::parse(".sb_count").unwrap();
    let total_results = document.select(&count_selector).next()
        .map(|e| e.text().collect::<String>());

    Ok(SerpData {
        results,
        people_also_ask: vec![], // Bing PAA is complex, skipping for now
        related_searches,
        featured_snippet: None,
        total_results,
    })
}

// Wrapper with Retry Logic
pub async fn search_google(keyword: &str) -> Result<SerpData> {
    println!("🔎 Starting Google Deep Search for: {}", keyword);
    let mut last_error = String::from("No results found");
    
    // Max 3 attempts for resilience
    for attempt in 1..=3 {
        if attempt > 1 {
             println!("🔄 Retry Attempt {}/3...", attempt);
        }

        match search_google_attempt(keyword).await {
            Ok(data) => {
                if data.results.is_empty() {
                    println!("⚠️ Attempt {}/3: Google returned 0 results (Block/Captcha?).", attempt);
                    if attempt < 3 {
                        let wait_time = 5 * attempt as u64;
                        println!("⏳ Waiting {}s before retry...", wait_time);
                        sleep(Duration::from_secs(wait_time)).await;
                        continue;
                    }
                } else {
                    println!("✅ Attempt {}/3: Success! Found {} results.", attempt, data.results.len());
                    return Ok(data);
                }
            }
            Err(e) => {
                println!("❌ Attempt {}/3: Error: {}", attempt, e);
                last_error = e.to_string();
                if attempt < 3 {
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
    
    Err(anyhow::anyhow!("Google search failed after 3 attempts. Last error: {}", last_error))
}

// Internal attempt function
async fn search_google_attempt(keyword: &str) -> Result<SerpData> {
    use rand::seq::SliceRandom;
    // Select a random User-Agent
    let user_agent = USER_AGENTS.choose(&mut rand::thread_rng())
        .unwrap_or(&"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36");
    
    println!("Using User-Agent: {}", user_agent);

    // Use anonymous/incognito mode (no profile persistence)
    let mut args = vec![
        std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
        std::ffi::OsStr::new("--no-sandbox"),
        std::ffi::OsStr::new("--disable-dev-shm-usage"),
        std::ffi::OsStr::new("--disable-infobars"),
        std::ffi::OsStr::new("--window-position=0,0"),
        std::ffi::OsStr::new("--ignore-certificate-errors"),
        std::ffi::OsStr::new("--ignore-certificate-errors-spki-list"),
    ];
    let ua_arg = format!("--user-agent={}", user_agent);
    args.push(std::ffi::OsStr::new(&ua_arg));

    // Add proxy if available (using new ProxyManager)
    let proxy_arg: String;
    let ext_arg: String;
    let current_proxy = PROXY_MANAGER.get_next_proxy();
    let _proxy_id = current_proxy.as_ref().map(|p| p.id.clone());
    
    if let Some(ref proxy) = current_proxy {
        println!("🔄 Using proxy: {} (healthy: {}, success_rate: {:.1}%)", 
            proxy.id, 
            proxy.healthy.load(std::sync::atomic::Ordering::Relaxed),
            proxy.success_rate() * 100.0
        );
        proxy_arg = format!("--proxy-server={}", proxy.to_chrome_arg());
        args.push(std::ffi::OsStr::new(&proxy_arg));
        
        // Add auth extension if proxy requires authentication
        if proxy.requires_auth() {
            let ext_path = generate_proxy_auth_extension(
                proxy.username.as_ref().unwrap(),
                proxy.password.as_ref().unwrap()
            );
            ext_arg = format!("--load-extension={}", ext_path);
            args.push(std::ffi::OsStr::new(&ext_arg));
            println!("🔐 Proxy auth extension loaded");
        }
    }

    let browser = Browser::new(LaunchOptions {
        headless: true,
        window_size: Some((1920, 1080)),
        args,
        ..Default::default()
    })?;

    let tab = browser.new_tab()?;

    // Layer 1: Device & Environment Fingerprinting (JS-Level)
    // Layer 1: Device & Environment Fingerprinting (JS-Level)
    let stealth_script = r#"
        Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
        Object.defineProperty(navigator, 'hardwareConcurrency', { get: () => 4 });
        
        // Canvas Noise
        const originalToDataURL = HTMLCanvasElement.prototype.toDataURL;
        HTMLCanvasElement.prototype.toDataURL = function(...args) {
             if (this.width > 0 && this.height > 0) {
                const context = this.getContext('2d');
                if (context) {
                    const imageData = context.getImageData(0, 0, this.width, this.height);
                    // Single pixel alpha modification for speed
                    if (imageData.data.length > 3) {
                         imageData.data[3] = Math.max(0, Math.min(255, imageData.data[3] + (Math.random() > 0.5 ? 1 : -1)));
                         context.putImageData(imageData, 0, 0);
                    }
                }
            }
            return originalToDataURL.apply(this, args); 
        };

        const getParameter = WebGLRenderingContext.prototype.getParameter;
        WebGLRenderingContext.prototype.getParameter = function(parameter) {
            if (parameter === 37445) return 'Intel Inc.';
            if (parameter === 37446) return 'Intel Iris OpenGL Engine';
            return getParameter.apply(this, [parameter]);
        };
        window.chrome = { runtime: {}, loadTimes: function() {}, csi: function() {}, app: {} };
        
        // Block WebRTC
        ['RTCPeerConnection', 'webkitRTCPeerConnection', 'mozRTCPeerConnection', 'msRTCPeerConnection'].forEach(className => {
             if (window[className]) window[className] = undefined;
        });
    "#;

    tab.enable_debugger()?;
    tab.call_method(headless_chrome::protocol::cdp::Page::AddScriptToEvaluateOnNewDocument {
        source: stealth_script.to_string(),
        world_name: None,
        include_command_line_api: None,
        run_immediately: None,
    })?;

    // 1. Navigate to Home
    println!("Navigating to Google Home...");
    tab.navigate_to("https://www.google.com/?hl=en")?;
    tab.wait_until_navigated()?;
    
    // Random wait to simulate reading
    sleep(Duration::from_millis(3000 + (rand::random::<u64>() % 2000))).await;

    // Handle consent page (if present)
    println!("Checking for consent page...");
    let consent_result = tab.evaluate(r#"
        (() => {
            if (document.body.textContent.includes('Before you continue') || 
                document.body.textContent.includes('Avant de continuer') ||
                document.body.textContent.includes('cookies')) {
                const acceptBtn = document.querySelector('button[id*="accept"], button[id*="agree"], button[id*="L2AGLb"], form[action*="consent"] button');
                if (acceptBtn) {
                    acceptBtn.click();
                    return "consent_clicked";
                }
                return "consent_found_no_button";
            }
            return "no_consent";
        })();
    "#, false)?;
    
    if let Some(serde_json::Value::String(result)) = consent_result.value {
        println!("Consent check result: {}", result);
        if result == "consent_clicked" {
            println!("Consent accepted, waiting for redirect...");
            sleep(Duration::from_secs(2)).await;
            tab.wait_until_navigated()?;
        }
    }
    
    // Human-like mouse movement (entropy)
    println!("Simulating human mouse movements...");
    let _ = tab.evaluate(r#"
        async function humanMouseMove(startX, startY, endX, endY, steps) {
            for (let i = 0; i <= steps; i++) {
                const t = i / steps;
                // Linear interpolation with jitter
                const x = startX + (endX - startX) * t + (Math.random() - 0.5) * 5;
                const y = startY + (endY - startY) * t + (Math.random() - 0.5) * 5;
                document.dispatchEvent(new MouseEvent('mousemove', {
                    view: window,
                    bubbles: true,
                    cancelable: true,
                    clientX: x,
                    clientY: y
                }));
                await new Promise(r => setTimeout(r, 10 + Math.random() * 20));
            }
        }
        // Move towards the search box (approx center of screen)
        humanMouseMove(100, 100, window.innerWidth/2, window.innerHeight/2 - 100, 30);
    "#, false)?;

    sleep(Duration::from_millis(1000)).await;
    
    // Take screenshot for debugging
    println!("Capturing screenshot for debugging...");
    if let Ok(screenshot) = tab.capture_screenshot(
        headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
        None,
        None,
        true
    ) {
        let _ = std::fs::write("debug/debug_google_screenshot.png", &screenshot);
        println!("Screenshot saved to debug/debug_google_screenshot.png");
    }

    // 2. Type Query (Layer 3: Typing Speed)
    // Google uses textarea[name='q'] or input[name='q'] depending on version/AB test. 
    // We try textarea first, then input.
    let search_box = match tab.wait_for_element("textarea[name='q']") {
        Ok(el) => el,
        Err(_) => tab.wait_for_element("input[name='q']")?,
    };
    
    search_box.click()?;
    
    // Clear any existing content (important for fresh search)
    println!("Clearing search box...");
    tab.evaluate(r#"
        const input = document.querySelector('textarea[name="q"]') || document.querySelector('input[name="q"]');
        if (input) { input.value = ''; input.focus(); }
    "#, false)?;
    sleep(Duration::from_millis(500)).await;
    
    // Type query naturally for personalized results (profile-based)
    println!("Typing query: {}...", keyword);
    for char in keyword.chars() {
        tab.type_str(&char.to_string())?;
        sleep(Duration::from_millis(100 + (rand::random::<u64>() % 150))).await;
    }
    
    sleep(Duration::from_millis(500)).await;

    // 3. Submit
    println!("Submitting search...");
    tab.press_key("Enter")?;
    tab.wait_until_navigated()?;
    println!("Search submitted.");
    
    // Check for Google autocorrection message and click "Search instead for [exact term]"
    // Wait longer for the "Search instead for" link to appear
    sleep(Duration::from_millis(3000)).await;
    let verbatim_result = tab.evaluate(r#"
        (() => {
            // Helper to find link by text
            const findLinkByText = (text) => {
                const links = document.querySelectorAll('a');
                for (const link of links) {
                    if (link.textContent.includes(text)) return link;
                }
                return null;
            };

            // 1. Look for "Search instead for" link
            const verbatimLink = document.querySelector('a.spell_orig') || 
                                  document.querySelector('a[href*="nfpr=1"]') ||
                                  document.querySelector('#fprsl') ||
                                  findLinkByText("Search instead for");
            
            if (verbatimLink) {
                console.log('[VERBATIM] Found original search link, clicking...');
                verbatimLink.click();
                return "clicked_verbatim";
            }

            // 2. Check for "Showing results for" (standard autocorrect)
            const showingFor = document.querySelector('.spell') || document.querySelector('#scl');
            if (showingFor) {
                const originalLink = showingFor.querySelector('a');
                if (originalLink) {
                    originalLink.click();
                    return "clicked_original";
                }
            }
            return "no_autocorrect";
        })();
    "#, false)?;
    
    if let Some(serde_json::Value::String(result)) = verbatim_result.value {
        println!("Verbatim check result: {}", result);
        if result != "no_autocorrect" {
            println!("Clicked verbatim link, waiting for reload...");
            sleep(Duration::from_secs(2)).await;
            tab.wait_until_navigated()?;
        }
    }

    // Layer 3: Behavioral Realism
    let _ = tab.evaluate(r#"
        async function humanMouseMove(startX, startY, endX, endY, steps) {
            for (let i = 0; i <= steps; i++) {
                const t = i / steps;
                const x = startX + (endX - startX) * t;
                const y = startY + (endY - startY) * t;
                document.dispatchEvent(new MouseEvent('mousemove', {
                    view: window, bubbles: true, cancelable: true, clientX: x, clientY: y
                }));
                await new Promise(r => setTimeout(r, 10 + Math.random() * 20));
            }
        }
        humanMouseMove(100, 100, 500, 400, 20);
    "#, false)?;
    
    sleep(Duration::from_millis(500)).await;

    let _ = tab.evaluate(r#"
        async function humanScroll() {
            const totalHeight = document.body.scrollHeight;
            let distance = 100;
            let scrolled = 0;
            while(scrolled < totalHeight) {
                window.scrollBy(0, distance);
                scrolled += distance;
                await new Promise(r => setTimeout(r, 100 + Math.random() * 300));
            }
            window.scrollBy(0, -200);
        }
        humanScroll();
    "#, true)?;

    // L3: Google Extraction Strategy (CDP-Based, Per Debug Sequence)
    // Step 1: ✅ Already navigating to homepage → typing → submit (not direct SERP URL)
    
    // Add static wait for Google JS to initialize before mutation observer
    println!("Waiting 3s for Google JS to initialize...");
    sleep(Duration::from_secs(3)).await;
    
    // Step 2: Mutation observer with increased timeout (15s) and logging
    println!("Waiting for Google DOM mutations to complete...");
    let wait_script = r#"
        new Promise((resolve) => {
            let timeout;
            let mutationCount = 0;
            const observer = new MutationObserver(() => {
                mutationCount++;
                console.log(`[MUTATION] Count: ${mutationCount}`);
                clearTimeout(timeout);
                timeout = setTimeout(() => {
                    console.log(`[MUTATION] Settled after ${mutationCount} mutations`);
                    observer.disconnect();
                    resolve("mutations_complete");
                }, 1000); // Increased debounce: 500ms → 1000ms
            });
            observer.observe(document.body, { childList: true, subtree: true });
            
            // Increased fallback timeout: 5s → 12s
            setTimeout(() => {
                console.log(`[MUTATION] Timeout reached after ${mutationCount} mutations`);
                observer.disconnect();
                resolve("timeout_reached");
            }, 12000);
        });
    "#;
    
    let wait_result = tab.evaluate(wait_script, true)?;
    println!("DOM wait result: {:?}", wait_result.value);
    
    // Step 3: Extract via semantic attributes (resilient to class changes)
    let extraction_method: String;
    let results: Vec<SearchResult>;
    
    // Method 1: DOM extraction using expanded selectors (Step 5)
    let dom_extract_script = r#"
        (() => {
            const results = [];
            const mainContent = document.querySelector('[role="main"]') || document.querySelector('#main');
            
            if (!mainContent) {
                console.log('[EXTRACT] No main content found');
                return JSON.stringify({method: "dom", results: [], error: "no_main"});
            }
            
            console.log('[EXTRACT] Main content found');
            
            // Step 5: Expanded selectors (union of known Google containers)
            const resultBlocks = mainContent.querySelectorAll(
                '[data-snf], .g, [jscontroller="SC7lYd"], [data-ved], .Gx5Zad'
            );
            
            console.log(`[EXTRACT] Found ${resultBlocks.length} result blocks`);
            
            // Step 4: DOM Snapshot Fallback
            if (resultBlocks.length === 0 && !document.querySelector('[role="main"] h3')) {
                console.log('[EXTRACT] No blocks found, trying script tag fallback');
                const scriptData = Array.from(document.scripts).find(s => 
                    s.textContent?.includes('"results":') || s.textContent?.includes('AF_initDataCallback')
                );
                if (scriptData) {
                    return JSON.stringify({
                        method: "script_fallback", 
                        results: [], 
                        raw_snippet: scriptData.textContent.substring(0, 200)
                    });
                }
            }
            
            resultBlocks.forEach((block, idx) => {
                const titleEl = block.querySelector('h3, [role="heading"]');
                const linkEl = block.querySelector('a[href^="http"]:not([href*="google.com"])') || 
                              block.querySelector('a[jsname]');
                const snippetEl = block.querySelector('[data-content], [role="text"], .VwiC3b, .IsZvec, .yXK7lf');
                
                if (titleEl && linkEl && linkEl.href && !linkEl.href.includes('google.com/search')) {
                    console.log(`[EXTRACT] Block ${idx}: ${titleEl.textContent.trim().substring(0, 30)}`);
                    results.push({
                        title: titleEl.textContent.trim(),
                        link: linkEl.href,
                        snippet: snippetEl ? snippetEl.textContent.trim() : ""
                    });
                }
            });
            
            console.log(`[EXTRACT] Returning ${results.length} results`);
            return JSON.stringify({method: "dom", results: results.slice(0, 10)});
        })();
    "#;
    
    match tab.evaluate(dom_extract_script, true) {
        Ok(result) => {
            if let Some(serde_json::Value::String(value_str)) = result.value {
                let parsed: serde_json::Value = serde_json::from_str(&value_str).unwrap_or_default();
                extraction_method = parsed["method"].as_str().unwrap_or("unknown").to_string();
                results = serde_json::from_value(parsed["results"].clone()).unwrap_or_default();
                println!("Extracted {} results via method: {}", results.len(), extraction_method);
            } else {
                extraction_method = "fallback".to_string();
                results = Vec::new();
            }
        }
        Err(e) => {
            eprintln!("DOM extraction failed: {}, trying JS context fallback", e);
            extraction_method = "js_context".to_string();
            
            // Method 2: JS Context fallback (window.google.search.cse)
            let js_extract_script = r#"
                (() => {
                    try {
                        const googleData = window.google?.search?.cse?.results?.[0]?.results || [];
                        return JSON.stringify({
                            method: "js_context",
                            results: googleData.slice(0, 10).map(r => ({
                                title: r.title || "",
                                link: r.url || "",
                                snippet: r.content || ""
                            }))
                        });
                    } catch(e) {
                        return JSON.stringify({method: "js_context", results: []});
                    }
                })();
            "#;
            
            match tab.evaluate(js_extract_script, true) {
                Ok(js_result) => {
                    if let Some(serde_json::Value::String(value_str)) = js_result.value {
                        let parsed: serde_json::Value = serde_json::from_str(&value_str).unwrap_or_default();
                        results = serde_json::from_value(parsed["results"].clone()).unwrap_or_default();
                    } else {
                        results = Vec::new();
                    }
                }
                Err(_) => {
                    results = Vec::new();
                }
            }
        }
    }
    
    println!("Extraction method: {}", extraction_method);
    
    println!("Found {} results.", results.len());

    if results.is_empty() {
        let html_content = tab.get_content().unwrap_or_default();
        eprintln!("Google returned 0 results. HTML len: {}", html_content.len());
        let _ = std::fs::write("debug/debug_google_tier1.html", &html_content);
    }

    // Extract People Also Ask
    let html_content = tab.get_content()?;
    let document = Html::parse_document(&html_content);
    
    let paa_selector = Selector::parse(".related-question-pair .s75CSd").unwrap();
    let mut people_also_ask: Vec<String> = Vec::new(); // Explicit type
    for element in document.select(&paa_selector) {
        if let Some(text) = element.text().next() {
            people_also_ask.push(text.to_string());
        }
    }

    // Extract Related Searches
    let related_selector = Selector::parse(".s75CSd, .k8XOCe, .related-searches-list a").unwrap();
    let mut related_searches: Vec<String> = Vec::new(); // Explicit type
    for element in document.select(&related_selector) {
         if let Some(text) = element.text().next() {
             let s = text.to_string();
             if s.len() > 3 {
                 related_searches.push(s);
             }
         }
    }

    // Extract Total Results
    let count_selector = Selector::parse("#result-stats").unwrap();
    let total_results = document.select(&count_selector).next()
        .map(|e| e.text().collect::<String>());
        
    // Extract Featured Snippet
    let snippet_selector = Selector::parse(".xpdopen .block-component, .c2xzTb").unwrap();
    let featured_snippet: Option<FeaturedSnippet> = document.select(&snippet_selector).next().map(|el| {
        FeaturedSnippet {
            content: el.text().collect::<String>(),
            source_url: None,
            source_title: None,
        }
    });

    Ok(SerpData {
        results,
        people_also_ask,
        related_searches,
        featured_snippet,
        total_results,
    })
}

pub async fn extract_content(url: &str) -> Result<ExtractedContent> {
    // Decode Bing/Google redirect URLs to get actual destination
    let actual_url = decode_search_url(url);
    println!("Extracting content from: {}", actual_url);
    
    // Use proper User-Agent and follow redirects
    use rand::seq::SliceRandom;
    let user_agent = USER_AGENTS.choose(&mut rand::thread_rng())
        .unwrap_or(&"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36");

    let client = reqwest::Client::builder()
        .user_agent(*user_agent)
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(Duration::from_secs(30))
        .build()?;
    
    let resp: reqwest::Response = client.get(&actual_url)
        .header("Accept-Language", "en-US,en;q=0.9")
        .send().await?;
    let final_url = resp.url().to_string();
    println!("Final URL after redirects: {}", final_url);
    
    let html = resp.text().await?;
    println!("Fetched HTML size: {} bytes", html.len());
    
    let mut reader = Cursor::new(html.as_bytes());
    
    // 1. Extract text with Readability
    let text = match readability::extractor::extract(&mut reader, &reqwest::Url::parse(&final_url)?) {
        Ok(product) => product.text,
        Err(_) => "Failed to extract content".to_string(),
    };

    // 2. Extract metadata manually using Scraper
    let document = Html::parse_document(&html);
    let desc_selector = Selector::parse("meta[name='description']").unwrap();
    let author_selector = Selector::parse("meta[name='author']").unwrap();
    let date_selector = Selector::parse("meta[property='article:published_time']").unwrap();

    let meta_description = document.select(&desc_selector).next()
        .and_then(|e| e.value().attr("content").map(|s| s.to_string()));
    
    let meta_author = document.select(&author_selector).next()
        .and_then(|e| e.value().attr("content").map(|s| s.to_string()));

    let meta_date = document.select(&date_selector).next()
        .and_then(|e| e.value().attr("content").map(|s| s.to_string()));

    Ok(ExtractedContent {
        html: html.clone(),
        text,
        meta_description,
        meta_author,
        meta_date,
    })
}

/// Deep extraction function that returns comprehensive WebsiteData using Headless Chrome
pub async fn extract_website_data(url: &str) -> Result<WebsiteData> {
    // Decode Bing/Google redirect URLs to get actual destination
    let actual_url = decode_search_url(url);
    println!("🔍 Deep integration extracting data from: {}", actual_url);
    
    use rand::seq::SliceRandom;
    let user_agent = USER_AGENTS.choose(&mut rand::thread_rng())
        .unwrap_or(&"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36");

    // Configure Chrome arguments for Stealth
    let mut args = vec![
        std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
        std::ffi::OsStr::new("--no-sandbox"),
        std::ffi::OsStr::new("--disable-dev-shm-usage"),
        std::ffi::OsStr::new("--disable-infobars"),
        std::ffi::OsStr::new("--window-position=0,0"),
        std::ffi::OsStr::new("--ignore-certificate-errors"),
        std::ffi::OsStr::new("--ignore-certificate-errors-spki-list"),
    ];
    let ua_arg = format!("--user-agent={}", user_agent);
    args.push(std::ffi::OsStr::new(&ua_arg));

    // Add proxy if available
    let current_proxy = PROXY_MANAGER.get_next_proxy();
    let proxy_arg: String;
    let ext_arg: String;
    
    if let Some(ref proxy) = current_proxy {
        proxy_arg = format!("--proxy-server={}", proxy.to_chrome_arg());
        args.push(std::ffi::OsStr::new(&proxy_arg));
        
        if proxy.requires_auth() {
            let ext_path = generate_proxy_auth_extension(
                proxy.username.as_ref().unwrap(),
                proxy.password.as_ref().unwrap()
            );
            ext_arg = format!("--load-extension={}", ext_path);
            args.push(std::ffi::OsStr::new(&ext_arg));
        }
    }

    // Launch Browser
    let browser = Browser::new(LaunchOptions {
        headless: true,
        window_size: Some((1920, 1080)),
        args,
        ..Default::default()
    })?;

    let tab = browser.new_tab()?;

    // Inject Stealth Script
    let stealth_script = r#"
        Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
        Object.defineProperty(navigator, 'hardwareConcurrency', { get: () => 4 });
        const originalToDataURL = HTMLCanvasElement.prototype.toDataURL;
        HTMLCanvasElement.prototype.toDataURL = function(...args) {
             if (this.width > 0 && this.height > 0) {
                const context = this.getContext('2d');
                if (context) {
                    const imageData = context.getImageData(0, 0, this.width, this.height);
                    if (imageData.data.length > 3) {
                         imageData.data[3] = Math.max(0, Math.min(255, imageData.data[3] + (Math.random() > 0.5 ? 1 : -1)));
                         context.putImageData(imageData, 0, 0);
                    }
                }
            }
            return originalToDataURL.apply(this, args); 
        };
        const getParameter = WebGLRenderingContext.prototype.getParameter;
        WebGLRenderingContext.prototype.getParameter = function(parameter) {
            if (parameter === 37445) return 'Intel Inc.';
            if (parameter === 37446) return 'Intel Iris OpenGL Engine';
            return getParameter.apply(this, [parameter]);
        };
        window.chrome = { runtime: {}, loadTimes: function() {}, csi: function() {}, app: {} };
        ['RTCPeerConnection', 'webkitRTCPeerConnection', 'mozRTCPeerConnection', 'msRTCPeerConnection'].forEach(className => {
             if (window[className]) window[className] = undefined;
        });
    "#;

    tab.enable_debugger()?;
    tab.call_method(headless_chrome::protocol::cdp::Page::AddScriptToEvaluateOnNewDocument {
        source: stealth_script.to_string(),
        world_name: None,
        include_command_line_api: None,
        run_immediately: None,
    })?;

    // Navigate
    println!("Navigating to: {}", actual_url);
    tab.navigate_to(&actual_url)?;
    
    // Use softer wait (wait for body) instead of strict load event to prevent timeouts on ads/tracking
    match tab.wait_for_element_with_custom_timeout("body", Duration::from_secs(15)) {
        Ok(_) => println!("Page body loaded."),
        Err(e) => println!("⚠️ Warning: Body wait timed out: {}. Attempting extraction anyway...", e),
    }

    // Wait for JS execution (Hydration)
    sleep(Duration::from_secs(4)).await;

    // Extract Data via JS
    let html = tab.evaluate("document.documentElement.outerHTML", false)?.value.unwrap().as_str().unwrap().to_string();
    let final_url = tab.get_url();
    let html_size = html.len() as u32;
    println!("Extracted HTML size via Browser: {} bytes", html_size);

    // Parse document using Scraper for consistency with previous logic
    let document = Html::parse_document(&html);
    
    // Extract base domain
    let base_domain = reqwest::Url::parse(&final_url)
        .map(|u| u.host_str().unwrap_or("").to_string())
        .unwrap_or_default();
    
    // 1. Extract title
    let title = tab.evaluate("document.title", false)?.value.unwrap().as_str().unwrap().to_string();
    
    // 2. Extract meta tags using Scraper
    let desc_selector = Selector::parse("meta[name='description']").unwrap();
    let keywords_selector = Selector::parse("meta[name='keywords']").unwrap();
    let author_selector = Selector::parse("meta[name='author']").unwrap();
    let date_selector = Selector::parse("meta[property='article:published_time']").unwrap();
    
    let meta_description = document.select(&desc_selector).next()
        .and_then(|e| e.value().attr("content").map(|s| s.to_string()));
    let meta_keywords = document.select(&keywords_selector).next()
        .and_then(|e| e.value().attr("content").map(|s| s.to_string()));
    let meta_author = document.select(&author_selector).next()
        .and_then(|e| e.value().attr("content").map(|s| s.to_string()));
    let meta_date = document.select(&date_selector).next()
        .and_then(|e| e.value().attr("content").map(|s| s.to_string()));
    
    // 3. Extract main text using Readability on the rendered HTML
    let mut reader = Cursor::new(html.as_bytes());
    let main_text = match readability::extractor::extract(&mut reader, &reqwest::Url::parse(&final_url)?) {
        Ok(product) => product.text,
        Err(_) => {
            // Fallback to body text if Readability fails
            tab.evaluate("document.body.innerText", false)
                .map(|v| v.value.unwrap().as_str().unwrap().to_string())
                .unwrap_or_default()
        },
    };
    let word_count = main_text.split_whitespace().count() as u32;
    
    // 4. Extract Schema.org/JSON-LD structured data
    let schema_org = extract_schema_org(&html);
    if !schema_org.is_empty() {
        println!("📊 Found {} Schema.org objects", schema_org.len());
    }
    
    // 5. Extract Open Graph data
    let (og_title, og_description, og_image, og_type) = extract_open_graph(&document);
    
    // 6. Extract contact information
    let emails = extract_emails(&html);
    let phone_numbers = extract_phone_numbers(&main_text);
    
    // 7. Extract images
    let images = extract_images(&document, &format!("https://{}", base_domain));
    
    // 8. Extract outbound links
    let outbound_links = extract_outbound_links(&document, &base_domain);
    
    Ok(WebsiteData {
        url: actual_url,
        final_url,
        title,
        meta_description,
        meta_keywords,
        meta_author,
        meta_date,
        main_text,
        html: html.clone(),
        word_count,
        html_size,
        schema_org,
        og_title,
        og_description,
        og_image,
        og_type,
        emails,
        phone_numbers,
        images,
        outbound_links,
    })
}

// Public function to decode Bing/Google redirect URLs to get actual destination
pub fn decode_search_url(url: &str) -> String {
    // Bing URLs: https://www.bing.com/ck/a?...&u=a1aHR0c...
    if url.contains("bing.com/ck/a") {
        if let Some(u_param) = url.split("&u=").nth(1) {
            let encoded = u_param.split('&').next().unwrap_or(u_param);
            // Remove 'a1' prefix if present
            let base64_part = if encoded.starts_with("a1") {
                &encoded[2..]
            } else {
                encoded
            };
            // Decode base64
            if let Ok(decoded) = base64_decode(base64_part) {
                if let Ok(decoded_str) = String::from_utf8(decoded) {
                    println!("Decoded Bing URL: {}", decoded_str);
                    return decoded_str;
                }
            }
        }
    }
    // Google URLs: https://www.google.com/url?...&url=https...
    if url.contains("google.com/url") {
        if let Some(url_param) = url.split("&url=").nth(1).or_else(|| url.split("?url=").nth(1)) {
            let decoded_url = urlencoding::decode(url_param.split('&').next().unwrap_or(url_param))
                .unwrap_or_else(|_| url_param.into())
                .to_string();
            return decoded_url;
        }
    }
    // Return original if not a redirect URL
    url.to_string()
}

// Simple base64 decoder
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    use std::collections::HashMap;
    
    let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut decode_map: HashMap<char, u8> = HashMap::new();
    for (i, c) in alphabet.chars().enumerate() {
        decode_map.insert(c, i as u8);
    }
    
    let input = input.trim_end_matches('=');
    let mut output = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits_collected = 0;
    
    for c in input.chars() {
        if let Some(&val) = decode_map.get(&c) {
            buffer = (buffer << 6) | val as u32;
            bits_collected += 6;
            if bits_collected >= 8 {
                bits_collected -= 8;
                output.push((buffer >> bits_collected) as u8);
                buffer &= (1 << bits_collected) - 1;
            }
        }
    }
    
    Ok(output)
}

// ============================================================================
// Generic Forum Crawler
// ============================================================================
pub async fn generic_crawl(url: &str, selectors: Option<std::collections::HashMap<String, String>>) -> Result<SerpData> {
    println!("🌐 Starting Generic Crawl for: {}", url);
    use rand::seq::SliceRandom;
    
    // Minimal browser setup for brevity (reusing user agent list from top of file)
    let user_agent = USER_AGENTS.choose(&mut rand::thread_rng())
        .unwrap_or(&"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36");

    let args = vec![
        std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
        std::ffi::OsStr::new("--no-sandbox"),
        std::ffi::OsStr::new("--disable-dev-shm-usage"),
        std::ffi::OsStr::new("--headless"),
        std::ffi::OsStr::new("--ignore-certificate-errors"),
    ];

    let browser = Browser::new(LaunchOptions {
        headless: true, 
        args,
        window_size: Some((1920, 1080)),
        ..Default::default()
    })?;

    let tab = browser.new_tab()?;
    tab.navigate_to(url)?;
    tab.wait_until_navigated()?;
    
    // Simulate scroll for forums (often lazy load)
    let _ = tab.evaluate("window.scrollTo(0, document.body.scrollHeight);", false);
    sleep(Duration::from_secs(2)).await;

    let html_content = tab.get_content()?;
    let document = Html::parse_document(&html_content);
    
    let mut results = Vec::new();
    let mut snippet_acc = String::new();

    if let Some(sel_map) = selectors {
        for (key, selector_str) in sel_map {
             if let Ok(selector) = Selector::parse(&selector_str) {
                 snippet_acc.push_str(&format!("--- {} ---\n", key));
                 for element in document.select(&selector) {
                     snippet_acc.push_str(&element.text().collect::<String>());
                     snippet_acc.push('\n');
                 }
             }
        }
    } else {
        // Default: Extract Title + H1
        snippet_acc.push_str("No selectors provided. Dumping title.\n");
        let title_sel = Selector::parse("title").unwrap();
        if let Some(t) = document.select(&title_sel).next() {
            snippet_acc.push_str(&t.text().collect::<String>());
        }
    }

    results.push(SearchResult {
        title: "Forum Data".to_string(),
        link: url.to_string(),
        snippet: snippet_acc,
    });

    Ok(SerpData {
        results,
        total_results: Some("1".to_string()),
        ..Default::default()
    })
}
