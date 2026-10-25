// The update check proxy. Both apps ask this Worker for the latest release, so they do not spend the
// 60 requests per hour that GitHub gives one IP. The answer stays in KV for 5 minutes.
// GitHub failure: the last good answer still goes out, marked x-cache: stale. No such answer: 502.
const RELEASE = "https://api.github.com/repos/spizganed/QuickBuds/releases/latest";
const KEY = "release:latest"; // fresh answer, 5 minutes
const STALE = "release:stale"; // last good answer, no expiry
const TTL = 300;

export async function latest(request, env) {
  const fresh = await read(env, KEY);
  if (fresh !== null) return answer(200, fresh);

  try {
    return answer(200, await store(env));
  } catch {
    const stale = await read(env, STALE);
    if (stale === null) return answer(502, { error: "The release list is not available. Try again later." });
    return answer(200, stale, { "x-cache": "stale" });
  }
}

/** Reads the GitHub answer, keeps the fields the apps parse, and caches it. */
async function store(env) {
  const res = await fetch(RELEASE, {
    headers: { Accept: "application/vnd.github+json", "User-Agent": "quickbuds-report" },
  });
  if (!res.ok) throw new Error(`GitHub: HTTP ${res.status}`);
  const release = map(await res.json());
  const body = JSON.stringify(release);
  await env.QUICKBUDS_KV.put(KEY, body, { expirationTtl: TTL });
  await env.QUICKBUDS_KV.put(STALE, body);
  return release;
}

/** The release fields the Android and desktop updaters read. Other fields are dropped. */
function map(json) {
  return {
    tag_name: json.tag_name ?? "",
    html_url: json.html_url ?? "",
    body: json.body ?? "",
    published_at: json.published_at ?? "",
    assets: Array.isArray(json.assets)
      ? json.assets.map((a) => ({ name: a.name ?? "", browser_download_url: a.browser_download_url ?? "" }))
      : [],
  };
}

/** The cached JSON, or null when the key is missing. */
async function read(env, key) {
  const body = await env.QUICKBUDS_KV.get(key);
  if (body === null) return null;
  try {
    return JSON.parse(body);
  } catch {
    return null;
  }
}

function answer(status, body, extra = {}) {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      "Content-Type": "application/json; charset=utf-8",
      "Cache-Control": "public, max-age=60",
      "Access-Control-Allow-Origin": "*",
      ...extra,
    },
  });
}
