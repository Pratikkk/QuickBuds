# Report Worker

This is the backend of the problem report. It is a Cloudflare Worker on the free plan. It uses a D1 database. The apps post
each report to `/report`. The Worker stores only reports with a known model, a description and, when the report has a log,
frames from the buds. A model test ("Test" category) always has the log. A crash report skips the model check and the description check. One sender can post 5 reports an hour.

## Deploy (once)

From this folder, with a free Cloudflare account:

```bash
npx wrangler login
npx wrangler d1 create quickbuds-reports        # copy the database_id into wrangler.toml
npx wrangler d1 execute quickbuds-reports --remote --file schema.sql
npx wrangler secret put READ_KEY                # a long random string: the key to read the reports
npx wrangler secret put SALT                    # another random string
npx wrangler deploy
```

`openssl rand -hex 24` makes a random string. A change to `src/index.js` or `models.json` needs only
`npx wrangler deploy`.

## Read the reports

Send the key in the `Authorization: Bearer <READ_KEY>` header. The Worker does not read the key from the URL, so a
browser link no longer works.

- The text page: `curl -H "Authorization: Bearer $READ_KEY" https://quickbuds-report.<subdomain>.workers.dev/`
- For a script: add `?format=json`, and keep the header.
- Only newer reports: add `?after=<id>`. More reports: add `?limit=100`.
- The apps send a daily count to `/ping`.
- Counts: add `?stats=1`, and `?days=<n>`.

The key is the only access control. Keep the key private.

## Mark a report

Each report has a `status`. The page shows only `new` and `working` reports.

- `new`: not read yet.
- `working`: a fix is in progress.
- `done`: fixed, or answered.
- `unsupported`: the buds are not a supported model.
- `no_log`: the log is missing or shows nothing.
- `no_description`: the description is empty or unclear.
- `needs_btsnoop`: only a btsnoop capture can show the cause.

```bash
npx wrangler d1 execute quickbuds-reports --remote --command "UPDATE reports SET status = 'done', note = 'fixed in 4.13.0' WHERE id = 7"
```

## Delete old reports

```bash
npx wrangler d1 execute quickbuds-reports --remote --command "DELETE FROM reports WHERE at < '2026-01-01'"
```

## Reply to a report

A report that carries an install ID can get one answer. Write it with SQL, 600 characters at most, plain text:

```bash
npx wrangler d1 execute quickbuds-reports --remote --command "UPDATE reports SET reply = 'Text' WHERE id = 42"
```

The next daily ping of that install returns the reply once. The app shows it in a sheet and the Worker marks it as seen.
A database made before replies needs the two `ALTER TABLE` lines in `schema.sql` first.
