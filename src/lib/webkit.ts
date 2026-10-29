// The page under WebKit (WebKitGTK on Linux, WKWebView on macOS) rather than WebView2 (Chromium, Windows): what WebKit
// is spared is keyed on `:root.webkit` in app.css.

/** Whether `ua` is WebKit's own engine, not Chromium (whose user agent says AppleWebKit too). */
export const isWebKit = (ua: string) => /AppleWebKit\//.test(ua) && !/(Chrome|Chromium|Edg)\//.test(ua);

/** Marks the page `webkit` under WebKit, before the first render. */
export function markWebKit(doc: Document = document, ua: string = navigator.userAgent) {
  if (isWebKit(ua)) doc.documentElement.classList.add('webkit');
}
