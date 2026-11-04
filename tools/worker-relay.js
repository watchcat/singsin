/**
 * SingSin — the reference-audio relay, as a Cloudflare Worker.
 *
 * words.hk serves one MP3 per Jyutping syllable but sends no
 * Access-Control-Allow-Origin header, so a browser cannot fetch those
 * bytes directly. This Worker fetches them server-side (where CORS does not
 * apply) and hands them back with a permissive CORS header.
 *
 * It is deliberately NOT an open proxy: only words.hk itself is relayed.
 *
 * Usage:  GET https://singsin-relay.<subdomain>.workers.dev/?url=<encoded>
 * Deploy: see README.md ("Deploying to Cloudflare").
 */

const ALLOWED_HOSTS = new Set(["words.hk", "www.words.hk"]);

const CORS_HEADERS = {
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "GET, HEAD, OPTIONS",
  "access-control-allow-headers": "*",
  "access-control-max-age": "86400",
};

function fail(message, status) {
  return new Response(message, {
    status,
    headers: { ...CORS_HEADERS, "content-type": "text/plain; charset=utf-8" },
  });
}

export default {
  async fetch(request) {
    if (request.method === "OPTIONS") {
      return new Response(null, { status: 204, headers: CORS_HEADERS });
    }
    if (request.method !== "GET" && request.method !== "HEAD") {
      return fail("Only GET is supported.", 405);
    }

    const target = new URL(request.url).searchParams.get("url");
    if (!target) {
      return fail("Missing the ?url= parameter.", 400);
    }

    let parsed;
    try {
      parsed = new URL(target);
    } catch {
      return fail("The ?url= parameter is not a valid URL.", 400);
    }
    if (parsed.protocol !== "https:" && parsed.protocol !== "http:") {
      return fail("Only http(s) URLs are relayed.", 400);
    }
    if (!ALLOWED_HOSTS.has(parsed.hostname)) {
      return fail("Host not allowed: " + parsed.hostname, 403);
    }

    let upstream;
    try {
      upstream = await fetch(parsed.toString(), {
        headers: { "user-agent": "SingSin/0.1 (Cantonese practice app)" },
        redirect: "follow",
      });
    } catch (err) {
      return fail("Upstream request failed: " + err, 502);
    }

    if (!upstream.ok) {
      return fail("Upstream returned " + upstream.status + ".", upstream.status);
    }

    // Buffer the body so the content-encoding / content-length headers that
    // came back with the original response cannot disagree with the bytes we
    // hand on. A syllable MP3 is a couple of kilobytes, so this is cheap.
    const body = await upstream.arrayBuffer();
    const contentType = upstream.headers.get("content-type") || "application/octet-stream";

    return new Response(body, {
      status: 200,
      headers: {
        ...CORS_HEADERS,
        "content-type": contentType,
        "content-length": String(body.byteLength),
        "cache-control": "public, max-age=86400",
      },
    });
  },
};
