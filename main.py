import argparse
import json
import logging
import os
import re
import time
from pathlib import Path
from typing import Any, Optional
from urllib.parse import urlencode

import requests
import urllib3

urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("ezlogin")


class PortalClient:
    def __init__(self, username: str, password: str, timeout: int = 10):
        self.username = username
        self.password = password
        self.timeout = timeout
        self.base_url = "https://192.168.200.127:8445"
        self.folder_name = "1606381611261/pc"
        self.custom_page_config_id = "ff808081760371a1017603ce291b008d"

        self.query_params = {
            "isPasscode": "N",
            "browserFlag": "zh",
            "folderName": self.folder_name,
            "httpsFlag": "Y",
            "publicBarcodeEncode": "null",
            "ssid": "edu_classroom",
            "url": "http://www.msftconnecttest.com/redirect",
            "authSuccess": "2",
            "redirectUrl": "",
            "urlParameter": "http://www.msftconnecttest.com/redirect",
            "currentTime": str(int(time.time() * 1000)),
            "authislogoff": "true",
        }

        self.auth_url = (
            f"{self.base_url}/PortalServer/customize/{self.folder_name}/auth.jsp?"
            f"{urlencode(self.query_params)}"
        )
        self.success_url = (
            f"{self.base_url}/PortalServer/customize/{self.folder_name}/success.jsp?"
            f"{urlencode(self.query_params)}"
        )

        self.session = requests.Session()
        self.session.verify = False
        self._ocr_engine = None
        self.session.headers.update(
            {
                "User-Agent": (
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
                    "AppleWebKit/537.36 (KHTML, like Gecko) "
                    "Chrome/146.0.0.0 Safari/537.36 Edg/146.0.0.0"
                ),
                "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
                "Connection": "keep-alive",
            }
        )

    def _sync_xsrf(self) -> None:
        token = self.session.cookies.get("XSRF-TOKEN")
        if token:
            self.session.headers["X-XSRF-TOKEN"] = token

    def _request(
        self,
        method: str,
        path_or_url: str,
        *,
        referer: Optional[str] = None,
        headers: Optional[dict] = None,
        data: Optional[dict] = None,
        allow_redirects: bool = True,
    ) -> requests.Response:
        self._sync_xsrf()
        req_headers = {}
        if referer:
            req_headers["Referer"] = referer
        if headers:
            req_headers.update(headers)

        url = path_or_url if path_or_url.startswith("http") else f"{self.base_url}{path_or_url}"
        resp = self.session.request(
            method=method,
            url=url,
            headers=req_headers,
            data=data,
            timeout=self.timeout,
            allow_redirects=allow_redirects,
        )
        logger.debug("HTTP %s %s -> %s", method.upper(), resp.request.path_url, resp.status_code)
        return resp

    @staticmethod
    def _json_or_none(resp: requests.Response) -> Optional[dict[str, Any]]:
        try:
            return resp.json()
        except (ValueError, json.JSONDecodeError):
            return None

    @staticmethod
    def _is_login_success(payload: Optional[dict[str, Any]], body: str) -> bool:
        if isinstance(payload, dict):
            data = payload.get("data")
            if isinstance(data, dict):
                portal_auth = data.get("portalAuth") is True
                status_ok = data.get("portalAuthStatus") == 0
                error_ok = data.get("portalErrorCode") in (None, 0)
                # 业务返回中 data 存在时，强制依赖 portalAuth 语义
                if portal_auth and status_ok and error_ok:
                    return True
                return False

            # 没有 data 字段时才允许老版 success 标记做兜底
            return payload.get("success") is True

        lowered = body.lower()
        return "success.jsp" in lowered or '"success":true' in lowered or '"code":0' in lowered

    @staticmethod
    def _login_message(payload: Optional[dict[str, Any]]) -> str:
        if not isinstance(payload, dict):
            return ""
        message = payload.get("message")
        return "" if message is None else str(message)

    def _build_valid_code_url(self) -> str:
        now_str = time.strftime("%a %b %d %Y %H:%M:%S GMT+0800 (China Standard Time)")
        return "/PortalServer/validCodeImg?" + urlencode(
            {
                "date": now_str,
                "includeLetter": "true",
                "folderName": self.folder_name,
                "httpsFlag": "Y",
            }
        )

    def fetch_captcha_image(self) -> bytes:
        resp = self._request("GET", self._build_valid_code_url(), referer=self.auth_url)
        if resp.status_code != 200 or not resp.content:
            raise requests.RequestException(f"failed to fetch captcha image: {resp.status_code}")
        return resp.content

    def _get_ocr_engine(self):
        if self._ocr_engine is not None:
            return self._ocr_engine

        try:
            import ddddocr  # type: ignore
        except ImportError as exc:
            raise RuntimeError(
                "auto captcha requires ddddocr. Install it with: pip install ddddocr"
            ) from exc

        self._ocr_engine = ddddocr.DdddOcr(show_ad=False)
        return self._ocr_engine

    def solve_captcha(self, image_bytes: bytes) -> str:
        engine = self._get_ocr_engine()
        raw = engine.classification(image_bytes)
        code = re.sub(r"[^A-Za-z0-9]", "", (raw or "")).strip()
        if len(code) > 4:
            code = code[:4]
        return code

    def init_session(self) -> None:
        logger.info("正在初始化门户会话...")
        self._request("GET", self.auth_url, referer=self.auth_url)
        self._request("GET", "/PortalServer/material/custom/custom.css", referer=self.auth_url)
        self._request("GET", "/PortalServer/material/custom/auth.js", referer=self.auth_url)
        self._request("GET", "/PortalServer/material/custom/lang/auth-zh.js", referer=self.auth_url)
        self._request("GET", self._build_valid_code_url(), referer=self.auth_url)

        ajax_headers = {
            "Accept": "*/*",
            "Content-Type": "application/x-www-form-urlencoded; charset=UTF-8",
            "Origin": self.base_url,
            "Sec-Fetch-Mode": "cors",
            "Sec-Fetch-Site": "same-origin",
            "X-Requested-With": "XMLHttpRequest",
        }
        self._request(
            "POST",
            "/PortalServer/Webauth/webAuthAction!getCustomPageConfig.action",
            referer=self.auth_url,
            headers=ajax_headers,
            data={"customPageConfigId": self.custom_page_config_id},
        )
        self._request(
            "GET",
            "/PortalServer/Webauth/thirdPartyAuthAction!getAppIdInfo.action",
            referer=self.auth_url,
            headers={
                "Accept": "*/*",
                "Sec-Fetch-Mode": "cors",
                "Sec-Fetch-Site": "same-origin",
            },
        )
        logger.info("门户会话初始化完成。")

    def login(self, valid_code: str = "") -> requests.Response:
        payload = {
            "authType": "",
            "userName": self.username,
            "password": self.password,
            "validCode": valid_code,
            "valideCodeFlag": "true",
            "authLan": "zh_CN",
            "hasValidateNextUpdatePassword": "true",
            "rememberPwd": "false",
            "browserFlag": "zh",
            "hasCheckCode": "false",
            "checkcode": "",
            "hasRsaToken": "false",
            "rsaToken": "",
            "autoLogin": "false",
            "userMac": "",
            "isBoardPage": "false",
            "disablePortalMac": "false",
            "overdueHour": "0",
            "overdueMinute": "0",
            "isAccountMsgAuth": "",
            "validCodeForAuth": "",
            "isAgreeCheck": "1",
        }
        headers = {
            "Accept": "*/*",
            "Content-Type": "application/x-www-form-urlencoded; charset=UTF-8",
            "Origin": self.base_url,
            "Sec-Fetch-Mode": "cors",
            "Sec-Fetch-Site": "same-origin",
            "X-Requested-With": "XMLHttpRequest",
        }
        return self._request(
            "POST",
            "/PortalServer/Webauth/webAuthAction!login.action",
            referer=self.auth_url,
            headers=headers,
            data=payload,
            allow_redirects=True,
        )

    def post_login_sync(self) -> None:
        headers = {
            "Accept": "*/*",
            "Content-Type": "application/x-www-form-urlencoded; charset=UTF-8",
            "Origin": self.base_url,
            "Sec-Fetch-Mode": "cors",
            "Sec-Fetch-Site": "same-origin",
            "X-Requested-With": "XMLHttpRequest",
        }

        self._request("GET", self.success_url, referer=self.auth_url)
        self._request(
            "POST",
            "/PortalServer/Webauth/webAuthAction!syncPortalAuthResult.action",
            referer=self.auth_url,
            headers=headers,
            data={"browserFlag": "zh", "userMac": ""},
        )
        self._request(
            "POST",
            "/PortalServer/Webauth/webAuthAction!getBindPolicy.action",
            referer=self.success_url,
            headers=headers,
            data={"browserFlag": "zh"},
        )



    def probe_connectivity(self, retries: int = 3, interval: float = 1.2) -> bool:
        probes = [
            "http://www.baidu.com",
            "https://www.baidu.com",
        ]
        for attempt in range(1, retries + 1):
            for url in probes:
                try:
                    resp = self.session.get(url, timeout=self.timeout, allow_redirects=True)
                    final_url = (resp.url or "").lower()
                    preview = resp.text[:120].replace("\n", " ") if resp.text else ""
                    preview_lower = preview.lower()

                    intercepted = (
                        "192.168.200.127" in final_url
                        or "portalserver" in final_url
                        or "portalserver" in preview_lower
                    )
                    passed = resp.status_code < 500 and not intercepted

                    logger.debug(
                        "PROBE attempt=%s url=%s status=%s final=%s intercepted=%s body=%s",
                        attempt,
                        url,
                        resp.status_code,
                        resp.url,
                        intercepted,
                        preview,
                    )

                    if passed:
                        return True
                except requests.RequestException as exc:
                    logger.warning("PROBE attempt=%s %s failed: %s", attempt, url, exc)

            if attempt < retries:
                time.sleep(interval)

        return False


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Huawei portal auto login runner")
    parser.add_argument("--username", default=os.getenv("EZLOGIN_USERNAME", ""))
    parser.add_argument("--password", default=os.getenv("EZLOGIN_PASSWORD", ""))
    parser.add_argument("--valid-code", default="", help="Captcha value if required")
    parser.add_argument("--auto-captcha", action="store_true", default=True, help="Auto solve captcha by OCR")
    parser.add_argument("--manual-captcha", dest="auto_captcha", action="store_false", help="Manual input captcha")
    parser.add_argument("--max-login-retries", type=int, default=5)
    parser.add_argument("--captcha-debug-dir", default="", help="Save captcha images for debugging OCR")
    parser.add_argument(
        "--probe-required",
        action="store_true",
        help="Treat connectivity probe failure as fatal (default: warning only)",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.username or not args.password:
        logger.error("Missing credentials. Use --username/--password or EZLOGIN_USERNAME/EZLOGIN_PASSWORD.")
        return 3

    client = PortalClient(args.username, args.password)

    try:
        client.init_session()
        login_resp = None
        ok = False

        for attempt in range(1, args.max_login_retries + 1):
            if args.valid_code and attempt == 1:
                valid_code = args.valid_code
                logger.info("Use provided captcha code for attempt %s", attempt)
            elif args.auto_captcha:
                image_bytes = client.fetch_captcha_image()
                if args.captcha_debug_dir:
                    debug_dir = Path(args.captcha_debug_dir)
                    debug_dir.mkdir(parents=True, exist_ok=True)
                    img_file = debug_dir / f"captcha_{int(time.time() * 1000)}.png"
                    img_file.write_bytes(image_bytes)

                valid_code = client.solve_captcha(image_bytes)
                logger.info("第 %s 次验证码识别结果: %s", attempt, valid_code)
            else:
                valid_code = input("请输入验证码: ").strip()

            if not valid_code:
                logger.warning("Empty captcha value at attempt %s", attempt)
                continue

            login_resp = client.login(valid_code=valid_code)
            login_json = client._json_or_none(login_resp)
            body = login_resp.text.lower()

            if login_json is not None:
                summary_success = login_json.get("success")
                summary_message = login_json.get("message")
                portal_status = None
                if isinstance(login_json.get("data"), dict):
                    portal_status = login_json["data"].get("portalAuthStatus")
                logger.info(
                    "登录响应摘要: success=%s, portalAuthStatus=%s, message=%s",
                    summary_success,
                    portal_status,
                    summary_message,
                )
                logger.debug("Login JSON detail: %s", login_json)

            ok = login_resp.status_code == 200 and client._is_login_success(login_json, body)

            if ok:
                break

            message = client._login_message(login_json)
            if message:
                logger.warning("Attempt %s failed: %s", attempt, message)
            else:
                logger.warning("Attempt %s failed, status=%s", attempt, login_resp.status_code)

            time.sleep(0.5)

        if not ok or login_resp is None:
            logger.error("Login failed after %s attempts.", args.max_login_retries)
            return 1

        logger.info("登录成功，正在执行登录后同步。")
        client.post_login_sync()

        online = client.probe_connectivity()
        if not online:
            if args.probe_required:
                logger.error("Portal API succeeded but connectivity probe failed. Likely not fully authenticated.")
                logger.error("建议：1) 提供验证码参数 --valid-code  2) 检查账号是否绑定终端/MAC  3) 把 Login JSON 发我继续定位")
                return 4

            logger.warning("连通性探测未通过（可能为误判），但门户登录已成功，按成功处理。")
            logger.warning("如需严格探测可加参数: --probe-required")
            return 0

        logger.info("外网连通性探测通过。")
        return 0
    except RuntimeError as exc:
        logger.error("%s", exc)
        return 5
    except requests.RequestException as exc:
        logger.error("Network error: %s", exc)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())