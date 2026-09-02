# Hosting the App Store site

The support/marketing page and privacy policy live in `/docs` at the
root of this project (not in this `app-store/` folder — that's just
metadata and screenshots now). This is deliberate: once you push this
whole project to GitHub as a repo named `bit`, GitHub Pages can serve
`/docs` directly, so the site lives in the same repo as the code —
no separate repo needed.

## One-time setup, once you've pushed `bit` to GitHub

1. Push this project to a GitHub repo named `bit` under your account
   (`IntScription`), if you haven't already
2. In that repo, go to **Settings → Pages**
3. Under "Build and deployment", set Source to **Deploy from a
   branch**, branch `main`, folder **/docs** → Save
4. Wait ~1 minute, then the site is live at:
   `https://intscription.github.io/bit/`
   and the privacy policy at:
   `https://intscription.github.io/bit/privacy-policy.html`

These are the exact URLs already used in `metadata.md` and hardcoded
into the app's Profile → About → Privacy Policy link (see
`ProfileView.swift`), so once Pages is enabled everything just works
— nothing else to update.

## Note on the old `bit-app-site` repo

Before this consolidation, a separate repo (`github.com/IntScription/
bit-app-site`) was created and is currently live with the same two
pages. Once `/docs` in the `bit` repo is live at the URLs above, that
old repo is redundant — ask Claude to delete it, or delete it
yourself from github.com/IntScription/bit-app-site/settings once
you've confirmed the new URLs work.
