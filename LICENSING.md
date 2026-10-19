# Licensing

QuickBuds is offered under two licences.

## Open source: GPL

Everyone can use, change and share QuickBuds under the GNU General Public License v3.0 or later.
See [LICENSE](./LICENSE). Contributions come in under the same licence.

## Commercial: on request

A company that cannot ship GPL code can ask for a commercial licence. The grant is made case by case.
Open an issue to start the request.

## What blocks a commercial grant

- **Slint.** The desktop app (`desktop/`) uses [Slint](https://slint.dev). Slint is GPL-3.0 or commercial.
  QuickBuds needs Slint's commercial licence before it can grant a commercial licence for the desktop app.
  The Android app (`app/`) does not use Slint.
- **Model values.** `docs/PROTOCOL.md` and `models.json` use our own names and our own text.
  The per-model values (feature bytes, status queries, gesture slots) come from vendor app data.
  A licence review must decide if these facts can sit under a commercial grant.
- **Fonts.** Doto and Google Sans Flex use the SIL Open Font License 1.1. They keep that licence.

## Trademarks

QuickBuds is not affiliated with OnePlus, OPPO, realme or HeyTap. Their names belong to their owners.
