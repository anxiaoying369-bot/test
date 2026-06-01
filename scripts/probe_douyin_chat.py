#!/usr/bin/env python3
from __future__ import annotations
import json, os, socket, sys, time
from pathlib import Path

CDP_PORT=9222
from pathlib import Path
import os
import sys

# Add current dir to path for compat
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from compat import get_chrome_path, get_data_dir

CHROME_PATH = get_chrome_path()
# 动态计算 cookie 路径
COOKIE_PATH = get_data_dir() / "cookies" / "douyin" / "抖音账号" / "cookie.json"
OUT = Path("/tmp/douyin_chat_probe.json")
if sys.platform == "win32":
    OUT = Path(os.environ.get("TEMP", "C:\\temp")) / "douyin_chat_probe.json"


def is_port_in_use(port):
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.settimeout(1); return s.connect_ex(("127.0.0.1", port)) == 0

def inject(page):
    data=json.loads(COOKIE_PATH.read_text())
    cookies=data.get('cookies') or []
    n=0
    for c in cookies:
        name=c.get('name'); value=c.get('value')
        if not name or value is None: continue
        domain=c.get('domain') or '.douyin.com'; path=c.get('path') or '/'
        try:
            page.run_cdp('Network.setCookie', **{'name':name,'value':str(value),'domain':domain,'path':path,'secure':bool(c.get('secure', False)),'httpOnly':bool(c.get('httpOnly') or c.get('http_only'))})
            n+=1
        except Exception:
            try:
                page.run_js(f"document.cookie = {json.dumps(f'{name}={value}; domain={domain}; path={path}', ensure_ascii=False)}")
                n+=1
            except Exception: pass
    return n, len(cookies)

from DrissionPage import ChromiumPage, ChromiumOptions
co=ChromiumOptions(); co.set_browser_path(CHROME_PATH)
if is_port_in_use(CDP_PORT): co.set_address(f"127.0.0.1:{CDP_PORT}")
else:
    co.set_argument(f"--remote-debugging-port={CDP_PORT}")
    co.set_argument(f"--user-data-dir={os.path.expanduser('~/chrome-debug-profile')}")
page=ChromiumPage(co)
try: page=page.latest_tab
except Exception: pass
page.get('https://www.douyin.com/')
time.sleep(1)
injected=inject(page)
page.get('https://www.douyin.com/chat')
time.sleep(8)
js=r'''
async function() {
  const simplify = (el) => {
    const r = el.getBoundingClientRect();
    return {
      tag: el.tagName.toLowerCase(),
      cls: el.className && String(el.className).slice(0,200),
      id: el.id || '',
      role: el.getAttribute('role') || '',
      dataE2e: el.getAttribute('data-e2e') || '',
      aria: el.getAttribute('aria-label') || '',
      title: el.getAttribute('title') || '',
      href: el.href || el.getAttribute('href') || '',
      text: (el.innerText || el.textContent || '').trim().replace(/\s+/g,' ').slice(0,300),
      rect: {x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height)},
      html: el.outerHTML.slice(0,1200)
    };
  };
  const all = Array.from(document.querySelectorAll('body *'));
  const visible = all.filter(el => {
    const r=el.getBoundingClientRect(); const st=getComputedStyle(el);
    return r.width>20 && r.height>10 && st.visibility!=='hidden' && st.display!=='none';
  });
  const left = visible.filter(el => { const r=el.getBoundingClientRect(); return r.x < Math.min(520, innerWidth*0.45) && r.y>40; });
  const clickable = visible.filter(el => el.matches('a,button,[role=button],[tabindex],li,div') && (el.innerText||el.textContent||el.getAttribute('aria-label')||'').trim()).slice(0,500);
  const classStats = {};
  for (const el of visible) {
    const cls=String(el.className||'').split(/\s+/).filter(Boolean).slice(0,4).join(' ');
    if (cls) classStats[cls]=(classStats[cls]||0)+1;
  }
  const scripts = Array.from(document.scripts).map(s => s.src).filter(Boolean).slice(0,50);
  const nextData = window._ROUTER_DATA || window.__INITIAL_STATE__ || window.__RENDER_DATA__ || null;
  const storage = {};
  for (let i=0;i<localStorage.length;i++){ const k=localStorage.key(i); if(/im|chat|conversation|message|user|douyin/i.test(k||'')) storage[k]=String(localStorage.getItem(k)).slice(0,2000); }
  return JSON.stringify({
    url: location.href,
    title: document.title,
    bodyText: document.body.innerText.slice(0,3000),
    injectedCookie: arguments[0],
    visibleCount: visible.length,
    left: left.map(simplify).slice(0,160),
    clickable: clickable.map(simplify).slice(0,220),
    classStats: Object.entries(classStats).sort((a,b)=>b[1]-a[1]).slice(0,80),
    scripts,
    routerKeys: nextData && typeof nextData === 'object' ? Object.keys(nextData).slice(0,50) : [],
    routerSample: nextData ? JSON.stringify(nextData).slice(0,5000) : '',
    storage,
  });
}
'''
result=page.run_js(js)
OUT.write_text(result if isinstance(result,str) else json.dumps(result,ensure_ascii=False,indent=2), encoding='utf-8')
print(OUT)
