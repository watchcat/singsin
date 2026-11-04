/**
 * SingSin — serves the built app from Cloudflare's edge, backed by R2.
 *
 * The bundle is stored in the `singsin-site` R2 bucket. The first request
 * for a file pulls it from the origin and writes it through to R2, so every
 * later request is served straight from Cloudflare storage.
 */

const ORIGIN = "https://ph3558utaau55.space.minimax.io";
const BROWSER_UA = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36";

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".wasm": "application/wasm",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".ico": "image/x-icon",
};

function typeFor(path) {
  const dot = path.lastIndexOf(".");
  if (dot < 0) return "application/octet-stream";
  return TYPES[path.slice(dot).toLowerCase()] || "application/octet-stream";
}

// Always declare the charset for text, so the Chinese is never sniffed.
function withCharset(type) {
  if (type.indexOf("text/") !== 0) return type;
  if (type.indexOf("charset") >= 0) return type;
  return type + "; charset=utf-8";
}

// Hashed assets are immutable and can be cached hard. `index.html` is the
// pointer to them, so it must always be revalidated — otherwise a redeploy
// would stay invisible for the whole cache window.
function cacheControl(key) {
  return key === "index.html" ? "no-cache, must-revalidate" : "public, max-age=3600";
}

// Remove the host's injected block (analytics + floating widget) from an HTML
// page, restoring the page we actually built.
function sanitizeHtml(text) {
  const marker = '<script data-minimax-analytics=';
  const i = text.indexOf(marker);
  if (i < 0) return text;
  const clean = text.slice(0, i).replace(/\n+$/, "");
  return clean + "</body>\n</html>\n";
}

export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    let path = decodeURIComponent(url.pathname);
    if (path === "/" || path === "") path = "/index.html";

    const key = path.replace(/^\/+/, "");

    // 1. Already in R2?
    let object = await env.SITE.get(key);
    if (object) {
      return new Response(object.body, {
        headers: {
          "content-type": withCharset(object.httpMetadata?.contentType || typeFor(path)),
          "cache-control": cacheControl(key),
          "x-content-type-options": "nosniff",
        },
      });
    }

    // 2. Otherwise pull it from the origin and write it through.
    let body;
    let type;
    try {
      const upstream = await fetch(ORIGIN + path, {
        headers: { "user-agent": BROWSER_UA, accept: "*/*" },
        redirect: "follow",
      });
      if (!upstream.ok) {
        return new Response("Not found: " + path, {
          status: upstream.status === 404 ? 404 : 502,
          headers: { "content-type": "text/plain; charset=utf-8" },
        });
      }
      body = await upstream.arrayBuffer();
      type = withCharset(upstream.headers.get("content-type") || typeFor(path));

      // The file host injects its own analytics snippet and a floating
      // widget into every page it serves. Strip them, so what we deliver
      // is exactly what we built.
      if (type.indexOf("text/html") === 0) {
        const text = new TextDecoder().decode(body);
        const clean = sanitizeHtml(text);
        if (clean !== text) {
          body = new TextEncoder().encode(clean).buffer;
        }
      }
    } catch (err) {
      return new Response("Origin unreachable: " + err, {
        status: 502,
        headers: { "content-type": "text/plain; charset=utf-8" },
      });
    }

    ctx.waitUntil(
      env.SITE.put(key, body, {
        httpMetadata: { contentType: type, cacheControl: cacheControl(key) },
      })
    );

    return new Response(body, {
      status: 200,
      headers: {
        "content-type": type,
        "content-length": String(body.byteLength),
        "cache-control": cacheControl(key),
        "x-content-type-options": "nosniff",
      },
    });
  },
};
