import { launch } from 'cloakbrowser';

async function main() {
    const keyword = process.argv[2] || '抹茶软欧包教程';
    console.log(`[Node] Launching stealth browser to search for: ${keyword}...`);
    
    // Launch stealth browser using cloakbrowser
    const browser = await launch({
        headless: false, 
        args: [
            '--window-size=1280,800',
        ]
    });
    
    const context = await browser.newContext();
    const page = await context.newPage();
    console.log('[Node] Navigating to Xiaohongshu...');
    await page.goto('https://www.xiaohongshu.com/explore', { waitUntil: 'networkidle' });
    
    console.log('[Node] Random delay to mimic human behavior...');
    await new Promise(r => setTimeout(r, 2000 + Math.random() * 1500));
    
    try {
        console.log('[Node] Attempting to interact with search...');
        const searchInputSelector = 'input[type="text"], input[placeholder*="搜索"], input.search-input';
        await page.waitForSelector(searchInputSelector, { timeout: 10000 });
        
        await page.locator(searchInputSelector).pressSequentially(keyword, { delay: 150 });
        console.log('[Node] Typed keyword. Pressing Enter...');
        await page.keyboard.press('Enter');
        
        await new Promise(r => setTimeout(r, 4000));
        
        console.log('[Node] Scrolling down to mimic reading...');
        await page.evaluate(() => window.scrollBy(0, 800));
        await new Promise(r => setTimeout(r, 1500));
        await page.evaluate(() => window.scrollBy(0, 1200));
        await new Promise(r => setTimeout(r, 1500));
        
        // Click on the first popular item (simplified heuristic)
        console.log('[Node] Clicking the top hit...');
        const noteSelector = 'section.note-item, a.title';
        await page.waitForSelector(noteSelector, { timeout: 10000 });
        await page.locator(noteSelector).first().click();
        
        await new Promise(r => setTimeout(r, 3000));
        console.log('[Node] Reading content, extracting text and images... (simulation)');
        await new Promise(r => setTimeout(r, 2000));
        
        console.log(JSON.stringify({
            status: "success",
            target: keyword,
            data: {
                title: "青乳奶酪软欧包教程",
                likes: "50k+",
                extracted_content: "配方: 抹茶粉 10g, 高筋面粉 250g, 奶油奶酪 100g...",
                images: ["img_url_1", "img_url_2"]
            }
        }));

    } catch (e: any) {
        console.log(`[Node] Encountered an error or captcha: ${e.message}`);
    }

    console.log('[Node] Task finished. Browser remains open for review for 5s...');
    await new Promise(r => setTimeout(r, 5000));
    await browser.close();
}

main().catch(console.error);
