#!/usr/bin/env python3
"""
抖音视频发布器 — DrissionPage 驱动 creator.douyin.com 创作者中心上传发布。

设计参考开源项目 dreammis/social-auto-upload 的抖音上传流程（选择器/步骤），
但把其 Playwright 实现移植为本项目统一的 DrissionPage，并复用已保存的登录 cookie：
  - 不接管用户的 9222 调试 Chrome（那是登录/采集用的），而是用**独立 user-data 目录**
    另起一个 Chrome，把账号目录里的 cookie.json 灌进去，从而以该账号身份打开创作者中心。
  - 一次性 CLI，按任务调用：上传 → 填标题/描述/话题 → （可选）定时 → 发布。
  - 进度以「行分隔 JSON」打到 stdout，由 Rust 读取后 emit 给前端；日志走 stderr。

用法：
  python3 douyin_publisher.py \
      --account-dir <cookies/douyin/<name>> \
      --video <a.mp4> --title "标题" \
      [--tags "话题1,话题2"] [--description "描述"] [--cover <jpg>] \
      [--location "城市"] [--schedule "2026-06-20 18:00"] [--headless]

stdout 协议（每行一个 JSON）：
  {"type":"progress","stage":"上传中","progress":40}
  {"type":"result","success":true,"url":"https://creator.douyin.com/..."}
  {"type":"result","success":false,"error":"cookie 已失效，请重新登录该账号"}
"""

import argparse
import json
import sys
import os
import time
import traceback
from datetime import datetime
from pathlib import Path

# compat / cookie 工具与本脚本不在同一目录（在 scripts/ 根），加进 sys.path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

try:
    from DrissionPage import ChromiumPage, ChromiumOptions
    from DrissionPage.common import Keys
except ImportError:
    print(json.dumps({"type": "result", "success": False,
                      "error": "DrissionPage 未安装，请运行 pip install DrissionPage"}), flush=True)
    sys.exit(1)

from compat import get_chrome_path  # noqa: E402

UPLOAD_URL = "https://creator.douyin.com/creator-micro/content/upload"
HOME_URL = "https://creator.douyin.com/"
DATE_FORMAT = "%Y-%m-%d %H:%M"


# ─── stdout 进度协议 ──────────────────────────────────────────────────────────

def emit(stage: str, progress: int) -> None:
    print(json.dumps({"type": "progress", "stage": stage, "progress": progress},
                     ensure_ascii=False), flush=True)


def result(success: bool, url: str = "", error: str = "") -> None:
    print(json.dumps({"type": "result", "success": success, "url": url, "error": error},
                     ensure_ascii=False), flush=True)


def log(msg: str) -> None:
    print(f"[publisher] {msg}", file=sys.stderr, flush=True)


# ─── DOM 小工具 ───────────────────────────────────────────────────────────────

def is_disabled(btn) -> bool:
    """按钮禁用态——只看**属性**不看 class（抖音 class 是构建期哈希，会变）：
    disabled 属性存在（可能是空串）或 aria-disabled='true'。"""
    if btn.attr('disabled') is not None:
        return True
    return (btn.attr('aria-disabled') or '').lower() == 'true'


def find_button(page, text: str):
    """按**精确**文本找 <button>（避免 text:发布 模糊命中「高清发布」等）。"""
    for b in page.eles('tag:button'):
        try:
            if (b.text or '').strip() == text:
                return b
        except Exception:
            continue
    return None


def dismiss_popups(page) -> None:
    """关掉首次使用时的「我知道了」引导气泡——它们会盖住并拦截发布按钮点击。"""
    for _ in range(5):
        btn = find_button(page, '我知道了')
        if not btn:
            return
        try:
            btn.click()
        except Exception:
            return
        time.sleep(0.4)


# ─── cookie 载入 ──────────────────────────────────────────────────────────────

def load_cookies(account_dir: Path) -> list:
    """从账号目录的 cookie.json 读取 cookie 列表（DrissionPage 保存的原始格式）。"""
    cookie_json = account_dir / "cookie.json"
    if not cookie_json.exists():
        raise FileNotFoundError(f"找不到 cookie.json：{cookie_json}，请先完成该账号登录")
    data = json.loads(cookie_json.read_text(encoding="utf-8"))
    cookies = data.get("cookies") or []
    if not cookies:
        raise ValueError("cookie.json 中没有 cookies，请重新登录该账号")
    # DrissionPage set.cookies 需要至少 name/value/domain；补 domain 兜底。
    for c in cookies:
        if isinstance(c, dict) and not c.get("domain"):
            c["domain"] = ".douyin.com"
    return cookies


# ─── 浏览器 ───────────────────────────────────────────────────────────────────

def build_browser(account_name: str, headless: bool) -> ChromiumPage:
    """用独立 user-data 目录另起 Chrome，避免与登录/采集用的 9222 实例冲突。"""
    data_dir = os.environ.get("AUTOCAST_DATA_DIR") or str(Path.home() / ".autocast")
    profile = Path(data_dir) / "publisher" / "chrome_profile" / account_name
    profile.mkdir(parents=True, exist_ok=True)

    co = ChromiumOptions()
    chrome_path = get_chrome_path()
    if chrome_path:
        co.set_browser_path(chrome_path)
    co.set_user_data_path(str(profile))
    co.auto_port()  # 自动分配端口，绝不碰 9222
    co.set_argument("--no-first-run")
    co.set_argument("--no-default-browser-check")
    co.set_argument("--window-size", "1280,900")
    if headless:
        co.headless(True)
    return ChromiumPage(co)


def is_logged_in(page) -> bool:
    """当前页若出现「登录」入口即视为未登录。"""
    for kw in ("手机号登录", "扫码登录", "二维码登录"):
        try:
            if page.ele(f"text:{kw}", timeout=1):
                return False
        except Exception:
            continue
    return True


# ─── 各步骤（选择器移植自 social-auto-upload 抖音上传器） ──────────────────────

def wait_publish_page(page, timeout: int = 60) -> None:
    """上传文件后会自动跳到发布编辑页（两种 URL 形态之一）。"""
    deadline = time.time() + timeout
    while time.time() < deadline:
        url = page.url or ""
        if "creator-micro/content/publish" in url or "creator-micro/content/post/video" in url:
            return
        time.sleep(0.5)
    raise TimeoutError("上传后未跳转到发布编辑页（可能上传失败或被风控）")


def fill_title_desc_tags(page, title: str, description: str, tags: list) -> None:
    """标题（30 字内）+ 作品描述（contenteditable）+ 话题（#tag）。"""
    # 标题输入框：作品描述区域内第一个 text input
    title_input = (page.ele('css:input[placeholder*="标题"]', timeout=8)
                   or page.ele('css:input[type="text"]', timeout=8))
    if title_input:
        title_input.clear()
        title_input.input(title[:30])
    else:
        log("未找到标题输入框，跳过标题")

    # 作品描述：富文本编辑器——用 contenteditable 属性定位（页面唯一），不用易变的 class
    editor = page.ele('css:[contenteditable="true"]', timeout=8)
    if editor:
        editor.click()
        # 全选 + 删除清空已有内容
        page.actions.key_down(Keys.CTRL).type('a').key_up(Keys.CTRL).type(Keys.DEL)
        page.actions.type(description or title)
        for tag in tags:
            page.actions.type(f" #{tag}")
            page.actions.type(' ')  # 空格触发话题联想确认
    else:
        log("未找到作品描述编辑器，跳过描述/话题")


def wait_upload_done(page, timeout: int = 600) -> None:
    """发布编辑页出现「重新上传」即代表视频上传完成。"""
    deadline = time.time() + timeout
    last = -1
    while time.time() < deadline:
        try:
            if page.ele('text:重新上传', timeout=1):
                emit("视频上传完成", 70)
                return
            if page.ele('text:上传失败', timeout=1):
                raise RuntimeError("抖音提示视频上传失败")
        except RuntimeError:
            raise
        except Exception:
            pass
        cur = int((time.time() - (deadline - timeout)) / timeout * 30) + 40
        if cur != last:
            emit("视频上传中", min(cur, 69))
            last = cur
        time.sleep(2)
    raise TimeoutError("等待视频上传完成超时")


def set_cover(page, cover_path: str) -> None:
    if not cover_path:
        return
    try:
        page.ele('text:选择封面', timeout=5).click()
        time.sleep(1)
        # 封面弹窗里的文件输入：限定在 role=dialog 内用 type=file 定位，避开上传页的视频 input
        inp = (page.ele('xpath://*[@role="dialog"]//input[@type="file"]', timeout=8)
               or page.ele('css:input[type="file"]', timeout=4))
        if inp:
            inp.input(cover_path)
            time.sleep(2)
            done = find_button(page, '完成')
            if done:
                done.click()
            emit("封面已设置", 80)
    except Exception as e:
        log(f"设置封面失败，跳过：{e}")


def set_location(page, location: str) -> None:
    if not location:
        return
    try:
        page.ele('text:输入地理位置', timeout=5).click()
        page.actions.type(Keys.BACKSPACE)
        time.sleep(2)
        page.actions.type(location)
        opt = page.ele('css:div[role="listbox"] [role="option"]', timeout=5)
        if opt:
            opt.click()
    except Exception as e:
        log(f"设置地理位置失败，跳过：{e}")


def _close_declaration_modal(page) -> None:
    """兜底关掉自主声明弹窗（取消 / 关闭 / ESC），避免它遮挡发布按钮。"""
    try:
        cancel = find_button(page, '取消')
        if cancel:
            cancel.click()
            return
        # 关闭按钮：用 aria-label（无障碍属性，比 class 稳）；再不行按 ESC
        close = page.ele('css:[aria-label="close"]', timeout=1) or page.ele('css:[aria-label="关闭"]', timeout=1)
        if close:
            close.click()
            return
        page.actions.type(Keys.ESCAPE)
    except Exception:
        pass


def set_self_declaration(page) -> None:
    """抖音「自主声明」弹窗：选「内容为个人观点或见解」→ 确定。

    关键：单选项的文字 span 常带 pointer-events:none，直接点文字不会选中，要点外层可点容器
    （label）。无论成功与否都**确保弹窗被关闭**，否则它会盖住整页、让后面的「发布」点击失效（实测踩坑）。
    """
    opened = False
    try:
        entry = page.ele('text:请选择自主声明', timeout=4) or page.ele('text:自主声明', timeout=2)
        if not entry:
            return
        entry.click()
        opened = True
        time.sleep(1)
        # 限定在弹窗(role=dialog)内，点含「个人观点」文本的外层 label（避开 pointer-events:none 的文字 span，
        # 也避开页面别处预览文案）。用 role + 文本定位，不依赖易变 class。
        radio = page.ele('xpath://*[@role="dialog"]//label[contains(., "个人观点")]', timeout=4)
        if not radio:
            log("自主声明：未找到「个人观点」选项，关闭弹窗跳过")
            _close_declaration_modal(page)
            return
        radio.click()
        time.sleep(0.5)
        # 选中后「确定」由 disabled 变可点
        confirm = None
        for b in page.eles('tag:button'):
            if (b.text or '').strip() == '确定' and not is_disabled(b):
                confirm = b
                break
        if confirm:
            confirm.click()
            time.sleep(0.8)
            log("自主声明已选「内容为个人观点或见解」")
        else:
            log("自主声明：确定按钮不可点，关闭弹窗跳过")
            _close_declaration_modal(page)
    except Exception as e:
        log(f"自主声明设置失败，关闭弹窗跳过：{e}")
        if opened:
            _close_declaration_modal(page)


def set_schedule(page, when: datetime) -> None:
    """切到「定时发布」并填入时间。"""
    try:
        radio = page.ele('text:定时发布', timeout=5)
        if radio:
            radio.click()
            time.sleep(1)
        # 用 placeholder 属性定位时间输入框，不依赖易变 class
        box = page.ele('css:input[placeholder="日期和时间"]', timeout=5) \
            or page.ele('css:[placeholder="日期和时间"]', timeout=2)
        if box:
            box.click()
            page.actions.key_down(Keys.CTRL).type('a').key_up(Keys.CTRL)
            page.actions.type(when.strftime(DATE_FORMAT))
            page.actions.type(Keys.ENTER)
            time.sleep(1)
        emit("已设置定时发布", 88)
    except Exception as e:
        raise RuntimeError(f"设置定时发布失败：{e}")


def click_publish(page, timeout: int = 90) -> str:
    """点击**精确**的「发布」按钮并等待跳转到作品管理页，返回最终 URL。"""
    deadline = time.time() + timeout
    while time.time() < deadline:
        dismiss_popups(page)  # 先清掉「我知道了」引导，否则会拦截点击

        # 发布前可能弹「请设置封面后再发布」，自动选第一个推荐封面。
        # 推荐封面缩略图没有稳定的 text/role 锚点，只能退而用 class——但**仅匹配语义前缀**
        # `recommendCover`（CSS-module 哈希在后缀，前缀的本地名跨版本稳定），不写死完整哈希类名。
        try:
            if page.ele('text:请设置封面后再发布', timeout=1):
                rec = page.ele('xpath://*[contains(@class,"recommendCover")]', timeout=2)
                if rec:
                    rec.click()
                    time.sleep(1)
                    ok = find_button(page, '确定')
                    if ok and not is_disabled(ok):
                        ok.click()
        except Exception:
            pass

        btn = find_button(page, '发布')
        if btn and not is_disabled(btn):
            try:
                btn.click()
            except Exception:
                # 被遮挡时退回 JS 点击
                try:
                    page.run_js('arguments[0].click()', btn)
                except Exception:
                    pass
        time.sleep(2)
        if "creator-micro/content/manage" in (page.url or ""):
            return page.url

    raise TimeoutError("点击发布后未跳转到作品管理页（可能仍在审核/被风控/有必填项未完成）")


# ─── 主流程 ───────────────────────────────────────────────────────────────────

def run(args) -> None:
    account_dir = Path(args.account_dir)
    account_name = account_dir.name
    video = Path(args.video)
    if not video.exists():
        result(False, error=f"视频文件不存在：{video}")
        return

    tags = [t.strip() for t in (args.tags or "").split(",") if t.strip()]
    schedule_dt = None
    if args.schedule:
        try:
            schedule_dt = datetime.strptime(args.schedule.strip(), DATE_FORMAT)
        except ValueError:
            result(False, error=f"定时时间格式应为 'YYYY-MM-DD HH:MM'：{args.schedule}")
            return

    emit("准备浏览器", 5)
    cookies = load_cookies(account_dir)
    page = None
    try:
        page = build_browser(account_name, headless=args.headless)

        emit("注入登录态", 12)
        page.get(HOME_URL)
        page.set.cookies(cookies)

        emit("打开创作者中心", 20)
        page.get(UPLOAD_URL)
        time.sleep(2)
        if not is_logged_in(page):
            result(False, error="cookie 已失效，请到「账号管理」重新登录该账号")
            return

        emit("上传视频文件", 30)
        # 上传页用 type=file 属性定位文件输入框（上传页此时只有这一个），不用易变 class
        file_input = page.ele('css:input[type="file"]', timeout=15)
        if not file_input:
            result(False, error="未找到上传入口（抖音创作者中心页面结构可能已变化）")
            return
        file_input.input(str(video.resolve()))

        wait_publish_page(page)
        emit("填写标题与话题", 50)
        fill_title_desc_tags(page, args.title, args.description or "", tags)

        wait_upload_done(page)

        set_location(page, args.location or "")
        set_cover(page, args.cover or "")
        set_self_declaration(page)

        if schedule_dt:
            set_schedule(page, schedule_dt)

        emit("提交发布", 92)
        final_url = click_publish(page)
        emit("发布成功", 100)

        # 注意：**绝不**把发布后浏览器的 cookie 回写到 cookie.json。
        # cookie.json 是账号管理/采集共用的登录态（含 www.douyin.com 全量 cookie + storage_state，
        # 登录流程专门访问首页/私信页采集 security-sdk）。发布浏览器在 creator.douyin.com 上下文
        # 拿到的是**残缺**子集，回写会覆盖并搞坏登录态（实测会把账号搞成未登录）。

        result(True, url=final_url)
    finally:
        if page is not None:
            try:
                page.quit()
            except Exception:
                pass


def main() -> None:
    parser = argparse.ArgumentParser(description="抖音视频发布器")
    parser.add_argument("--account-dir", required=True, help="账号 cookie 目录 cookies/douyin/<name>")
    parser.add_argument("--video", required=True, help="待发布的视频文件路径")
    parser.add_argument("--title", required=True, help="标题（30 字内）")
    parser.add_argument("--tags", default="", help="话题，逗号分隔")
    parser.add_argument("--description", default="", help="作品描述")
    parser.add_argument("--cover", default="", help="封面图片路径（可选）")
    parser.add_argument("--location", default="", help="地理位置（可选）")
    parser.add_argument("--schedule", default="", help="定时发布时间 'YYYY-MM-DD HH:MM'（可选，留空=立即）")
    parser.add_argument("--headless", action="store_true", help="无界面后台发布")
    args = parser.parse_args()

    try:
        run(args)
    except Exception as e:
        log(traceback.format_exc())
        result(False, error=str(e))


if __name__ == "__main__":
    main()
