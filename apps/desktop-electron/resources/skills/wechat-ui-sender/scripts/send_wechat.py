import argparse
import os
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

try:
    import pyautogui
    import pyperclip
    import pygetwindow
except ImportError as exc:
    sys.stderr.write(
        "Missing dependency: {0}. Install requirements with `pip install pyautogui pyperclip pygetwindow pillow`.\n".format(exc.name)
    )
    sys.exit(2)

pyautogui.FAILSAFE = True

DEFAULT_WINDOW_TITLE = "微信"
DEFAULT_PAUSE = 0.8

WECHAT_PATHS = [
    os.path.expandvars(r"%ProgramFiles%\Tencent\WeChat\WeChat.exe"),
    os.path.expandvars(r"%ProgramFiles(x86)%\Tencent\WeChat\WeChat.exe"),
    os.path.expandvars(r"%LocalAppData%\Programs\WeChat\WeChat.exe"),
    r"C:\Program Files\Tencent\WeChat\WeChat.exe",
    r"C:\Program Files (x86)\Tencent\WeChat\WeChat.exe",
    r"D:\Program Files\Tencent\WeChat\WeChat.exe",
]


def is_wechat_running() -> bool:
    """Check if WeChat process is running (including minimized to tray)."""
    try:
        import psutil
        for proc in psutil.process_iter(['name']):
            if proc.info.get('name', '').lower() == 'wechat.exe':
                return True
    except Exception:
        pass

    try:
        result = subprocess.run(
            ['tasklist', '/FI', 'IMAGENAME eq WeChat.exe'],
            capture_output=True, text=True, timeout=5
        )
        return 'WeChat.exe' in result.stdout
    except Exception:
        pass

    return False


def launch_wechat(max_wait: float = 30.0) -> bool:
    """Attempt to launch WeChat from known paths."""
    # Detach flags for Windows so the child does not inherit our stdio pipes.
    # Without this, Node.js spawn's 'close' event never fires because the
    # launched process keeps the pipe handles open.
    _detach_kwargs = dict(
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP | subprocess.DETACHED_PROCESS,
    ) if sys.platform == 'win32' else dict(
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )

    for path in WECHAT_PATHS:
        if os.path.exists(path):
            try:
                subprocess.Popen([path], **_detach_kwargs)
                print(f"Launched WeChat from: {path}")
                return True
            except Exception as e:
                print(f"Failed to launch {path}: {e}")
                continue

    try:
        subprocess.Popen(
            'start "" "wechat"',
            shell=True,
            **_detach_kwargs,
        )
        print("Attempted to launch WeChat via start command")
        return True
    except Exception:
        pass

    return False


def restore_from_tray(window_title: str, pause: float = DEFAULT_PAUSE) -> bool:
    """Try to restore WeChat window from tray/minimized state using multiple methods."""
    methods = [
        lambda: pyautogui.hotkey('win', 'd'),
        lambda: pyautogui.hotkey('win', 'm'),
        lambda: pyautogui.hotkey('win', 'up'),
    ]

    for i, method in enumerate(methods):
        try:
            method()
            time.sleep(1.0)
            windows = pygetwindow.getWindowsWithTitle(window_title)
            if windows and windows[0].width > 100:
                return True
        except Exception:
            continue

    return False


def activate_wechat(
    window_title: str,
    pause: float = DEFAULT_PAUSE,
    force_launch: bool = False,
    max_retries: int = 3,
    retry_delay: float = 2.0
) -> pygetwindow.Window:
    for attempt in range(1, max_retries + 1):
        windows = pygetwindow.getWindowsWithTitle(window_title)

        if windows:
            window = windows[0]
            try:
                if window.isMinimized:
                    try:
                        window.restore()
                    except Exception:
                        pyautogui.hotkey('alt', 'tab')
                        time.sleep(0.5)
                else:
                    window.activate()
            except Exception:
                try:
                    window.restore()
                except Exception:
                    pass

            time.sleep(pause)

            if window.width > 100 and window.height > 100:
                return window

        if attempt < max_retries:
            print(f"WeChat not found (attempt {attempt}/{max_retries}), checking process...")

            if not is_wechat_running():
                print("WeChat process not running, attempting to launch...")
                if not launch_wechat():
                    print("Failed to launch WeChat automatically")
                else:
                    print(f"Waiting {retry_delay}s for WeChat to start...")
                    time.sleep(retry_delay)
            else:
                print(f"WeChat running but window not found, trying to restore from tray...")
                restore_from_tray(window_title, pause)
                time.sleep(retry_delay)
        else:
            raise RuntimeError(
                f"Could not activate WeChat window after {max_retries} attempts. "
                f"Please ensure WeChat is open and visible (not hidden in tray)."
            )

    raise RuntimeError(f"Could not find a window matching title '{window_title}'.")


def focus_contact(contact: str, search_delay: float = 1.0, pause: float = DEFAULT_PAUSE) -> None:
    pyautogui.hotkey("ctrl", "f")
    time.sleep(pause)
    pyperclip.copy(contact)
    pyautogui.hotkey("ctrl", "v")
    time.sleep(search_delay)


def select_contact_with_retry(
    contact: str,
    max_retries: int = 3,
    pause: float = DEFAULT_PAUSE,
) -> bool:
    """
    Select a contact from search results, trying multiple results if needed.
    Returns True if successful, False otherwise.
    """
    keywords = [k.strip().lower() for k in contact.split() if len(k.strip()) >= 2]

    for attempt in range(max_retries):
        if attempt > 0:
            print(f"尝试第 {attempt + 1} 个搜索结果...")
            for _ in range(attempt):
                pyautogui.press("down")
                time.sleep(0.2)
            pyautogui.press("enter")
            time.sleep(pause)

        time.sleep(0.5)

        try:
            windows = pygetwindow.getWindowsWithTitle("微信")
            if windows:
                window = windows[0]
                title = window.title.lower()
                if any(kw in title for kw in keywords):
                    print(f"✓ 找到正确的聊天窗口: {window.title}")
                    return True
        except Exception:
            pass

    return False


def select_contact(
    confirm: bool = False,
    confirm_delay: float = 2.0,
    pause: float = DEFAULT_PAUSE,
    result_index: int = 0,
    interactive: bool = False
) -> None:
    if confirm or interactive:
        print(f"\n⏸️  搜索结果已显示，请手动选择正确的联系人/群...")
        print(f"   - 使用 ↑/↓ 箭头选择")
        print(f"   - 选中后按 Enter 继续发送消息")
        input(f"   ▶️  按 Enter 继续 (或直接选择并按 Enter)...")
    if result_index > 0 and not interactive:
        for _ in range(result_index):
            pyautogui.press("down")
            time.sleep(0.2)
    pyautogui.press("enter")
    time.sleep(pause)


def detect_and_skip_web_search(pause: float = DEFAULT_PAUSE) -> bool:
    """
    Detect if the first search result is "网络搜索" and skip it.
    Returns True if skipped, False if first result was already a contact.
    """
    try:
        from PIL import Image
        import pytesseract

        screenshot = pyautogui.screenshot()

        left, top = 200, 100
        width, height = 400, 300

        if screenshot.size[0] < left + width or screenshot.size[1] < top + height:
            return False

        search_area = screenshot.crop((left, top, left + width, top + height))

        text = pytesseract.image_to_string(search_area, lang='chi_sim+eng')

        if "网络搜索" in text or "Web Search" in text or "网络搜索" in text.replace(" ", ""):
            print("检测到'网络搜索'结果，按 ↓ 跳过...")
            pyautogui.press("down")
            time.sleep(0.3)
            pyautogui.press("enter")
            time.sleep(pause)
            return True

        return False
    except ImportError:
        print("提示: 安装 pytesseract Pillow 可启用自动检测'网络搜索'功能")
        return False
    except Exception:
        return False


def focus_input_box(window: pygetwindow.Window, offset_x: int = 200, offset_y: int = 550) -> None:
    pyautogui.click(x=window.left + offset_x, y=window.top + offset_y)
    time.sleep(DEFAULT_PAUSE)


def focus_input_box_center(window: pygetwindow.Window, x_ratio: float = 0.5, y_ratio: float = 0.85) -> None:
    center_x = window.left + int(window.width * x_ratio)
    center_y = window.top + int(window.height * y_ratio)
    pyautogui.click(x=center_x, y=center_y)
    time.sleep(DEFAULT_PAUSE)


def send_message(message: str, pause: float = DEFAULT_PAUSE) -> None:
    pyperclip.copy(message)
    time.sleep(0.3)
    pyautogui.hotkey("ctrl", "v")
    time.sleep(pause)
    pyautogui.press("enter")


def read_markdown_file(file_path: str) -> str:
    """Read markdown content from a file and return as plain text for sending."""
    path = Path(file_path)
    if not path.exists():
        raise FileNotFoundError(f"Markdown file not found: {file_path}")

    if path.suffix.lower() not in ['.md', '.markdown', '.txt']:
        raise ValueError(f"File must be a markdown file (.md, .markdown, .txt): {file_path}")

    content = path.read_text(encoding='utf-8')
    return content.strip()


def capture_screenshot(output_dir: Path) -> Path:
    output_dir.mkdir(parents=True, exist_ok=True)
    path = output_dir / f"wechat_ui_{datetime.utcnow().strftime('%Y%m%dT%H%M%SZ')}.png"
    pyautogui.screenshot(str(path))
    return path


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Send a WeChat message via UI automation. ONLY supports markdown files.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python send_wechat.py "Contact Name" --file message.md
  python send_wechat.py ".NET AI/Agents" --file "D:\\messages\\news.md"
  python send_wechat.py "Group" --file msg.md --auto-retry  # 自动尝试多个结果直到找到正确的
  python send_wechat.py "Group" --file msg.md --auto-detect-web-search  # 自动跳过网络搜索

NOTE: This script only accepts markdown files. To send a message:
1. Write your content to a .md file first
2. Pass the absolute file path using --file argument
        """
    )
    parser.add_argument(
        "contact",
        help="Display name or keyword for the contact/group search box"
    )
    parser.add_argument(
        "--file",
        type=str,
        required=True,
        help="Path to a markdown file (.md, .markdown, .txt) containing the message to send"
    )
    parser.add_argument(
        "--window-title",
        default=DEFAULT_WINDOW_TITLE,
        help=f"Window title to match (default: '{DEFAULT_WINDOW_TITLE}')",
    )
    parser.add_argument(
        "--search-delay",
        type=float,
        default=1.0,
        help="Seconds to wait after typing contact before pressing Enter",
    )
    parser.add_argument(
        "--confirm",
        action="store_true",
        help="After search, wait for manual selection before sending message",
    )
    parser.add_argument(
        "--confirm-delay",
        type=float,
        default=2.0,
        help="Seconds to wait when --confirm is used (default: 2.0)",
    )
    parser.add_argument(
        "--typing-delay",
        type=float,
        default=DEFAULT_PAUSE,
        help="Seconds to wait after pasting message before sending",
    )
    parser.add_argument(
        "--pause",
        type=float,
        default=DEFAULT_PAUSE,
        help="General pause between actions in seconds",
    )
    parser.add_argument(
        "--input-offset-x",
        type=int,
        default=200,
        help="X offset from window left to click for input box focus",
    )
    parser.add_argument(
        "--input-offset-y",
        type=int,
        default=550,
        help="Y offset from window top to click for input box focus",
    )
    parser.add_argument(
        "--screenshot-dir",
        type=Path,
        default=None,
        help="Optional folder to dump a screenshot after sending",
    )
    parser.add_argument(
        "--use-center",
        action="store_true",
        help="Use window center position for input box instead of fixed offset",
    )
    parser.add_argument(
        "--input-x-ratio",
        type=float,
        default=0.5,
        help="X ratio (0-1) for input box position when --use-center (default: 0.5)",
    )
    parser.add_argument(
        "--input-y-ratio",
        type=float,
        default=0.85,
        help="Y ratio (0-1) for input box position when --use-center (default: 0.85)",
    )
    parser.add_argument(
        "--force-launch",
        action="store_true",
        help="Force attempt to launch WeChat if not already running",
    )
    parser.add_argument(
        "--max-retries",
        type=int,
        default=3,
        help="Maximum retry attempts to find/activate WeChat window (default: 3)",
    )
    parser.add_argument(
        "--retry-delay",
        type=float,
        default=3.0,
        help="Seconds to wait between retry attempts (default: 3.0)",
    )
    parser.add_argument(
        "--check-only",
        action="store_true",
        help="Only check if WeChat is running, do not send message",
    )
    parser.add_argument(
        "--result-index",
        type=int,
        default=0,
        help="Index of search result to select (0=first, 1=second, etc.). Useful when search shows web results first",
    )
    parser.add_argument(
        "--interactive",
        action="store_true",
        help="After search, wait for user to select the correct contact manually (press Enter when ready)",
    )
    parser.add_argument(
        "--auto-detect-web-search",
        action="store_true",
        help="Automatically detect and skip '网络搜索' result in search dropdown",
    )
    parser.add_argument(
        "--auto-retry",
        action="store_true",
        help="Automatically try multiple search results until correct contact is found",
    )
    parser.add_argument(
        "--auto-retry-count",
        type=int,
        default=3,
        help="Number of results to try with --auto-retry (default: 3)",
    )

    args = parser.parse_args()

    if args.check_only:
        running = is_wechat_running()
        print(f"WeChat running: {running}")
        windows = pygetwindow.getWindowsWithTitle(args.window_title) if running else []
        print(f"Window visible: {len(windows) > 0}")
        sys.exit(0 if running else 1)

    message = read_markdown_file(args.file)
    print(f"Loaded message from file: {message[:100]}..." if len(message) > 100 else f"Loaded message from file: {message}")

    try:
        if args.force_launch and not is_wechat_running():
            print("Force launching WeChat...")
            launch_wechat()
            time.sleep(args.retry_delay)

        window = activate_wechat(
            args.window_title,
            args.pause,
            args.force_launch,
            args.max_retries,
            args.retry_delay
        )
        focus_contact(args.contact, args.search_delay, args.pause)
        if args.auto_retry:
            print("自动尝试多个搜索结果...")
            success = select_contact_with_retry(args.contact, args.auto_retry_count, args.pause)
            if not success:
                raise RuntimeError(f"在 {args.auto_retry_count} 个结果中未找到匹配的联系人/群")
        elif args.auto_detect_web_search:
            detect_and_skip_web_search(args.pause)
        else:
            select_contact(args.confirm, args.confirm_delay, args.pause, args.result_index, args.interactive)
        send_message(message, args.typing_delay)
        screenshot_path = None
        if args.screenshot_dir:
            screenshot_path = capture_screenshot(args.screenshot_dir)
    except Exception as exc:
        capture_path = None
        if args.screenshot_dir:
            try:
                capture_path = capture_screenshot(Path(args.screenshot_dir))
            except Exception:
                capture_path = None
        sys.stderr.write(f"ERROR: {exc}\n")
        if capture_path:
            sys.stderr.write(f"UI capture saved to: {capture_path}\n")
        sys.exit(1)

    print("status=success")
    if args.screenshot_dir and screenshot_path:
        print(f"screenshot={screenshot_path}")


if __name__ == "__main__":
    main()
