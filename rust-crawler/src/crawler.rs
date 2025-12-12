use headless_chrome::{Browser, LaunchOptions};
use anyhow::Result;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::time::Duration;
use tokio::time::sleep;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use once_cell::sync::Lazy;

// Global Proxy Manager
static PROXY_MANAGER: Lazy<ProxyManager> = Lazy::new(|| {
    ProxyManager::new(vec![
        // Add your proxies here: "scheme://ip:port"
        // "socks5://127.0.0.1:9050".to_string(), // Tor example
        // "http://user:pass@1.2.3.4:8080".to_string(),
    ])
});

struct ProxyManager {
    proxies: Vec<String>,
    current_index: AtomicUsize,
}

impl ProxyManager {
    fn new(proxies: Vec<String>) -> Self {
        Self {
            proxies,
            current_index: AtomicUsize::new(0),
        }
    }

    fn get_next_proxy(&self) -> Option<String> {
        if self.proxies.is_empty() {
            return None;
        }
        let index = self.current_index.fetch_add(1, Ordering::SeqCst) % self.proxies.len();
        Some(self.proxies[index].clone())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub title: String,
    pub link: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Default)]
pub struct ExtractedContent {
    pub html: String,
    pub text: String,
    pub meta_description: Option<String>,
    pub meta_author: Option<String>,
    pub meta_date: Option<String>,
}

pub async fn search_bing(keyword: &str) -> Result<Vec<SearchResult>> {
    // Use anonymous/incognito mode (no profile persistence)
    let mut args = vec![
        std::ffi::OsStr::new("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36"),
        std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
        std::ffi::OsStr::new("--no-sandbox"),
        std::ffi::OsStr::new("--disable-dev-shm-usage"),
        std::ffi::OsStr::new("--disable-infobars"),
        std::ffi::OsStr::new("--window-position=0,0"),
        std::ffi::OsStr::new("--ignore-certificate-errors"),
        std::ffi::OsStr::new("--ignore-certificate-errors-spki-list"),
    ];

    // Add proxy if available
    let proxy_arg; // Keep alive
    if let Some(proxy) = PROXY_MANAGER.get_next_proxy() {
        println!("Using proxy: {}", proxy);
        proxy_arg = format!("--proxy-server={}", proxy);
        args.push(std::ffi::OsStr::new(&proxy_arg));
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
    let stealth_script = r#"
        // 1. Remove navigator.webdriver
        Object.defineProperty(navigator, 'webdriver', {
            get: () => undefined,
        });

        // 2. Spoof Hardware Concurrency
        Object.defineProperty(navigator, 'hardwareConcurrency', {
            get: () => 4,
        });

        // 3. Canvas Noise (Simple Perlin-like jitter)
        const originalToDataURL = HTMLCanvasElement.prototype.toDataURL;
        HTMLCanvasElement.prototype.toDataURL = function(...args) {
            // Add subtle noise if it's a fingerprinting attempt (heuristics could be added here)
            // For now, we just call original to avoid breaking valid images, 
            // but in a real Tier 1, we'd manipulate the context data slightly before this.
            return originalToDataURL.apply(this, args);
        };
        
        // 4. WebGL Vendor Spoofing (Basic)
        const getParameter = WebGLRenderingContext.prototype.getParameter;
        WebGLRenderingContext.prototype.getParameter = function(parameter) {
            // UNMASKED_VENDOR_WEBGL
            if (parameter === 37445) {
                return 'Intel Inc.';
            }
            // UNMASKED_RENDERER_WEBGL
            if (parameter === 37446) {
                return 'Intel Iris OpenGL Engine';
            }
            return getParameter.apply(this, [parameter]);
        };
        
        // 5. Chrome Runtime (Mocking)
        window.chrome = {
            runtime: {},
            // Add other chrome properties as needed
        };
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
    
    println!("Typing query...");
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
        async function humanMouseMove(startX, startY, endX, endY, steps) {
            for (let i = 0; i <= steps; i++) {
                const t = i / steps;
                // Linear interpolation for now, can be upgraded to Bezier
                const x = startX + (endX - startX) * t;
                const y = startY + (endY - startY) * t;
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
        humanMouseMove(100, 100, 500, 400, 20);
    "#, false)?;
    
    sleep(Duration::from_millis(500)).await;

    // Gradual Scroll with pauses
    let _ = tab.evaluate(r#"
        async function humanScroll() {
            const totalHeight = document.body.scrollHeight;
            let distance = 100;
            let scrolled = 0;
            while(scrolled < totalHeight) {
                window.scrollBy(0, distance);
                scrolled += distance;
                // Random pause between scrolls
                await new Promise(r => setTimeout(r, 100 + Math.random() * 300));
            }
            // Scroll back up a bit
            window.scrollBy(0, -200);
        }
        humanScroll();
    "#, true)?; // await_promise = true
    
    // Wait for JavaScript to render results (same as Google fix)
    println!("Waiting for Bing DOM mutations to complete...");
    let wait_script = r#"
        new Promise((resolve) => {
            let timeout;
            let mutationCount = 0;
            const observer = new MutationObserver(() => {
                mutationCount++;
                clearTimeout(timeout);
                timeout = setTimeout(() => {
                    observer.disconnect();
                    resolve("mutations_complete");
                }, 1000);
            });
            observer.observe(document.body, { childList: true, subtree: true });
            setTimeout(() => {
                observer.disconnect();
                resolve("timeout_reached");
            }, 8000);
        });
    "#;
    let wait_result = tab.evaluate(wait_script, true)?;
    println!("Bing DOM wait result: {:?}", wait_result.value);
    
    // Take screenshot for debugging
    println!("Capturing Bing screenshot...");
    if let Ok(screenshot) = tab.capture_screenshot(
        headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
        None,
        None,
        true
    ) {
        let _ = std::fs::write("debug_bing_screenshot.png", &screenshot);
        println!("Screenshot saved to debug_bing_screenshot.png");
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
        let _ = std::fs::write("debug_bing_challenge.html", &html_content);
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
        let _ = std::fs::write("debug_bing_tier1.html", &html_content);
        
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
            .open("crawl_failures.log")
            .and_then(|mut f| std::io::Write::write_all(&mut f, log_entry.as_bytes()));
    }

    Ok(results)
}

pub async fn search_google(keyword: &str) -> Result<Vec<SearchResult>> {
    // Use anonymous/incognito mode (no profile persistence)
    let mut args = vec![
        std::ffi::OsStr::new("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36"),
        std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
        std::ffi::OsStr::new("--no-sandbox"),
        std::ffi::OsStr::new("--disable-dev-shm-usage"),
        std::ffi::OsStr::new("--disable-infobars"),
        std::ffi::OsStr::new("--window-position=0,0"),
        std::ffi::OsStr::new("--ignore-certificate-errors"),
        std::ffi::OsStr::new("--ignore-certificate-errors-spki-list"),
    ];

    // Add proxy if available
    let proxy_arg; // Keep alive
    if let Some(proxy) = PROXY_MANAGER.get_next_proxy() {
        println!("Using proxy: {}", proxy);
        proxy_arg = format!("--proxy-server={}", proxy);
        args.push(std::ffi::OsStr::new(&proxy_arg));
    }

    let browser = Browser::new(LaunchOptions {
        headless: true,
        window_size: Some((1920, 1080)),
        args,
        ..Default::default()
    })?;

    let tab = browser.new_tab()?;

    // Layer 1: Device & Environment Fingerprinting (JS-Level)
    let stealth_script = r#"
        Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
        Object.defineProperty(navigator, 'hardwareConcurrency', { get: () => 4 });
        const originalToDataURL = HTMLCanvasElement.prototype.toDataURL;
        HTMLCanvasElement.prototype.toDataURL = function(...args) { return originalToDataURL.apply(this, args); };
        const getParameter = WebGLRenderingContext.prototype.getParameter;
        WebGLRenderingContext.prototype.getParameter = function(parameter) {
            if (parameter === 37445) return 'Intel Inc.';
            if (parameter === 37446) return 'Intel Iris OpenGL Engine';
            return getParameter.apply(this, [parameter]);
        };
        window.chrome = { runtime: {} };
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
    
    // Take screenshot for debugging
    println!("Capturing screenshot for debugging...");
    if let Ok(screenshot) = tab.capture_screenshot(
        headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
        None,
        None,
        true
    ) {
        let _ = std::fs::write("debug_google_screenshot.png", &screenshot);
        println!("Screenshot saved to debug_google_screenshot.png");
    }

    // 2. Type Query (Layer 3: Typing Speed)
    // Google uses textarea[name='q'] or input[name='q'] depending on version/AB test. 
    // We try textarea first, then input.
    let search_box = match tab.wait_for_element("textarea[name='q']") {
        Ok(el) => el,
        Err(_) => tab.wait_for_element("input[name='q']")?,
    };
    
    search_box.click()?;
    
    // Type query naturally for personalized results (profile-based)
    println!("Typing query...");
    for char in keyword.chars() {
        tab.type_str(&char.to_string())?;
        sleep(Duration::from_millis(80 + (rand::random::<u64>() % 120))).await;
    }
    
    // 3. Submit
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
        let _ = std::fs::write("debug_google_tier1.html", &html_content);
    }

    Ok(results)
}

pub async fn extract_content(url: &str) -> Result<ExtractedContent> {
    // Decode Bing/Google redirect URLs to get actual destination
    let actual_url = decode_search_url(url);
    println!("Extracting content from: {}", actual_url);
    
    // Use proper User-Agent and follow redirects
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(Duration::from_secs(30))
        .build()?;
    
    let resp = client.get(&actual_url).send().await?;
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
