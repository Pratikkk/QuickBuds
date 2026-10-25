// The problem-report Worker. The apps POST /report as JSON; the developer reads GET / with the key in the
// Authorization header.
// It stores only reports the apps would send: a known model, a description and, when a log is on, frames from the buds.
import catalog from "../../app/src/main/assets/models.json";

const MODELS = new Set(catalog.whiteList.map((m) => m.name.toLowerCase()));
// "Test": a model test (Test my earbuds). It always carries the log.
const CATEGORIES = new Set(["UI", "Lag", "Connection", "Feature not working", "Battery", "Other", "Test"]);
const MAX_LOG = 1_000_000; // characters; a D1 value holds up to 2 MB
const MAX_TEXT = 5_000;
const PER_HOUR = 5; // reports per sender
// An RX frame line, as the Android and desktop packet logs write it.
const RX_FRAME = /\bRX(\[batch\])?:?\s+AA\b/i;

export default {
  async fetch(req, env) {
    const url = new URL(req.url);
    if (req.method === "POST" && url.pathname === "/report") return report(req, env);
    if (req.method === "POST" && url.pathname === "/ping") return ping(req, env);
    if (req.method === "GET" && url.pathname === "/") return list(req, url, env);
    return new Response("Not found", { status: 404 });
  },
};

/** Checks and stores one report. The answer text is shown to the person in the app. */
async function report(req, env) {
  let r;
  try {
    r = await req.json();
  } catch {
    return answer(400, "The report is not valid JSON.");
  }
  const model = str(r.model, 200);
  const description = str(r.description, MAX_TEXT);
  const app = str(r.app, 200);
  const crash = r.crash === true;
  const appId = APP_ID.test(str(r.id, 36).toLowerCase()) ? str(r.id, 36).toLowerCase() : null;
  const categories = Array.isArray(r.categories) ? r.categories.filter((c) => CATEGORIES.has(c)) : [];
  const log = typeof r.log === "string" ? r.log.slice(-MAX_LOG) : null;

  if (!crash && !MODELS.has(model.toLowerCase()))
    return answer(422, "QuickBuds supports OnePlus, OPPO and realme earbuds only. Pick your model from the list.");
  if (categories.includes("Test") && log === null)
    return answer(422, "A model test needs the log. Run the test again.");
  if (!crash && log !== null && !RX_FRAME.test(log))
    return answer(422, "The log is empty. Connect your earbuds first, then send the report.");
  if (!crash && !description) return answer(422, "Describe the problem: what you did and what happened.");

  const sender = await hash(env.SALT + (req.headers.get("CF-Connecting-IP") ?? ""));
  const since = new Date(Date.now() - 3600_000).toISOString();
  const { n } = await env.DB.prepare("SELECT COUNT(*) AS n FROM reports WHERE sender = ? AND at > ?")
    .bind(sender, since).first();
  if (n >= PER_HOUR) return answer(429, "Too many reports. Try again in an hour.");

  await env.DB.prepare(
    "INSERT INTO reports (at, sender, app, model, categories, description, crash, log, app_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
  ).bind(new Date().toISOString(), sender, app, model, categories.join(", "), description, crash ? 1 : 0, log, appId).run();
  return answer(200, "Sent. Thank you.");
}

/**
 * The developer's page: the newest reports first, as plain text, or as JSON with format=json.
 * Only "new" and "working" reports show. after=<id> shows only newer reports; limit=<n> (default 20, at most 100).
 */
async function list(req, url, env) {
  // The key comes only in the Authorization header (Bearer), so no URL holds it and no log keeps it.
  const given = req.headers.get("Authorization")?.replace(/^Bearer\s+/i, "");
  if (!env.READ_KEY || given !== env.READ_KEY) return new Response("Not found", { status: 404 });
  if (url.searchParams.has("stats")) return stats(url, env);
  const after = Number(url.searchParams.get("after") ?? 0) || 0;
  const limit = Math.min(Number(url.searchParams.get("limit") ?? 20) || 20, 100);
  const { results } = await env.DB.prepare(
    "SELECT id, at, app, app_id, model, categories, description, crash, log, status, note FROM reports WHERE id > ? AND status IN ('new', 'working') ORDER BY id DESC LIMIT ?",
  ).bind(after, limit).all();

  if (url.searchParams.get("format") === "json") return Response.json(results);
  const text = results.length === 0 ? "No reports.\n" : results.map((r) => [
    `#${r.id}  ${r.at}  ${r.app}${r.crash ? "  CRASH" : ""}  [${r.status}${r.note ? `: ${r.note}` : ""}]`,
    `Install: ${r.app_id ?? "-"}`,
    `Model: ${r.model}`,
    `Categories: ${r.categories || "-"}`,
    `Description:\n${r.description || "-"}`,
    `Log:\n${r.log ?? "(not sent)"}`,
  ].join("\n")).join(`\n\n${"=".repeat(72)}\n\n`) + "\n";
  return new Response(text, { headers: { "Content-Type": "text/plain; charset=utf-8" } });
}

function str(v, max) {
  return typeof v === "string" ? v.trim().slice(0, max) : "";
}

function answer(status, message) {
  return Response.json({ ok: status === 200, message }, { status });
}

async function hash(s) {
  const d = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(s));
  return [...new Uint8Array(d)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

const PLATFORMS = new Set(["Android", "Windows", "Linux"]);

const APP_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

/**
 * One app start per platform and day: the app sends {platform, version, id} at most once a day. `id` is a random
 * install ID the app made itself: it ties to no device, account or address. The Worker keeps the ID, the platform, the
 * version, the first and the last day seen, and one row per ID and day. An older app sends no ID: it only adds 1 to the day's counter.
 */
async function ping(req, env) {
  let r;
  try {
    r = await req.json();
  } catch {
    return answer(400, "Not valid JSON.");
  }
  const platform = str(r.platform, 20);
  const version = str(r.version, 20);
  const id = str(r.id, 36).toLowerCase();
  if (!PLATFORMS.has(platform) || !/^\d+(\.\d+){1,3}$/.test(version)) return answer(422, "Unknown platform or version.");
  const day = new Date().toISOString().slice(0, 10);
  const batch = [env.DB.prepare(
    "INSERT INTO daily_use (day, platform, version, n) VALUES (?, ?, ?, 1) ON CONFLICT (day, platform, version) DO UPDATE SET n = n + 1",
  ).bind(day, platform, version)];
  if (APP_ID.test(id)) {
    batch.push(
      env.DB.prepare(
        "INSERT INTO installs (id, platform, version, first_day, last_day) VALUES (?, ?, ?, ?, ?) " +
          "ON CONFLICT (id) DO UPDATE SET platform = excluded.platform, version = excluded.version, last_day = excluded.last_day",
      ).bind(id, platform, version, day, day),
      env.DB.prepare("INSERT OR IGNORE INTO activity (day, id) VALUES (?, ?)").bind(day, id),
    );
  }
  await env.DB.batch(batch);
  const reply = APP_ID.test(id) ? await takeReply(env, id) : null;
  return Response.json({ ok: true, message: "ok", ...(reply && { reply }) });
}

/**
 * The developer's answer to a report of this install, once: the oldest unread one. The app shows it in a sheet.
 * A failed read never fails the ping. The text is the `reply` column of the report (set with SQL, see the README).
 */
async function takeReply(env, id) {
  try {
    const row = await env.DB.prepare(
      "SELECT id, reply FROM reports WHERE app_id = ? AND reply IS NOT NULL AND reply_seen = 0 ORDER BY id LIMIT 1",
    ).bind(id).first();
    if (!row) return null;
    await env.DB.prepare("UPDATE reports SET reply_seen = 1 WHERE id = ?").bind(row.id).run();
    return { id: row.id, text: String(row.reply).slice(0, 600) };
  } catch {
    return null;
  }
}

/**
 * The developer's counts: GET /?stats=1[&days=30] with the key in the Authorization header. Per day and platform: the
 * active installs (distinct IDs), the new installs, and the ping count (which includes apps without an ID). Totals:
 * installs, seen in the last 7 and 30 days.
 */
async function stats(url, env) {
  const days = Math.min(Number(url.searchParams.get("days") ?? 30) || 30, 365);
  const since = new Date(Date.now() - days * 86400_000).toISOString().slice(0, 10);
  const daily = await env.DB.prepare(
    "SELECT u.day AS day, u.platform AS platform, SUM(u.n) AS pings, " +
      "(SELECT COUNT(*) FROM activity a JOIN installs i ON i.id = a.id WHERE a.day = u.day AND i.platform = u.platform) AS active, " +
      "(SELECT COUNT(*) FROM installs i WHERE i.first_day = u.day AND i.platform = u.platform) AS new_installs " +
      "FROM daily_use u WHERE u.day >= ? GROUP BY u.day, u.platform ORDER BY u.day DESC, u.platform",
  ).bind(since).all();
  const d7 = new Date(Date.now() - 7 * 86400_000).toISOString().slice(0, 10);
  const d30 = new Date(Date.now() - 30 * 86400_000).toISOString().slice(0, 10);
  const totals = await env.DB.prepare(
    "SELECT COUNT(*) AS installs, SUM(last_day >= ?) AS seen_7d, SUM(last_day >= ?) AS seen_30d FROM installs",
  ).bind(d7, d30).first();
  if (url.searchParams.get("format") === "json") return Response.json({ totals, daily: daily.results });
  const lines = daily.results.map((r) => `${r.day}  ${r.platform.padEnd(8)} active ${String(r.active).padStart(4)}  new ${String(r.new_installs).padStart(4)}  pings ${r.pings}`);
  return new Response(`installs ${totals.installs}  seen 7d ${totals.seen_7d ?? 0}  seen 30d ${totals.seen_30d ?? 0}\n\n${lines.join("\n")}\n`, {
    headers: { "Content-Type": "text/plain; charset=utf-8" },
  });
}
