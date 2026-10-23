# Google sign-in: the one-time setup

Folio's users click **Continue with Google**, pick their account in the
browser, and are done. For that, Folio has to be registered with Google once,
by whoever builds it. Users never see these steps.

## 1. Register Folio with Google (about 5 minutes)

At <https://console.cloud.google.com>, signed in with your Google account:

1. **Create a project.** Use the project picker at the top, then
   **New project**, and name it `Folio`.
2. **Turn on the Drive API.** Go to APIs & Services › Library, search
   "Google Drive API", and click **Enable**.
3. **Set up the consent screen.** Go to APIs & Services › OAuth consent
   screen (on newer consoles: Google Auth Platform › Branding), then:
   - User type: **External**.
   - App name: `Folio`, plus your support email. A logo is optional:
     `assets/brand` has one.
   - Scopes (Data access): add `.../auth/drive.file`. This lets Folio see
     only the files it creates, nothing else in anyone's Drive. Google rates
     it non-sensitive, so no security review is needed.
   - Audience: click **Publish app** (In production). While the app is in
     "Testing", only test users you list can sign in, and their sign-in
     expires every 7 days.
4. **Create the client.** Go to APIs & Services › Credentials ›
   **Create credentials › OAuth client ID**, choose **Desktop app**, and
   name it `Folio`. Copy the **Client ID** and the **Client secret**.

## 2. Build it in

Google offers the client as a JSON file (`client_secret_….json`). Never
commit it or its values: the repository is public, and GitHub and Google
both flag a client secret pushed to a public repository. `.gitignore`
keeps `client_secret_*.json` and `google-client.env` out.

Give the two values to the build in one of two ways. The environment wins
over the file.

- **Environment variables**, for builds in a Claude Code cloud session or
  CI: set `FOLIO_GOOGLE_CLIENT_ID` and `FOLIO_GOOGLE_CLIENT_SECRET` in the
  environment's settings (the cloud environment menu in the session's
  title bar › Edit › environment variables). Every new session's builds
  then have them.
- **`google-client.env`** at the repository root, for builds on your own
  computer:

  ```
  FOLIO_GOOGLE_CLIENT_ID=1234567890-abc.apps.googleusercontent.com
  FOLIO_GOOGLE_CLIENT_SECRET=GOCSPX-...
  ```

  The JSON's `installed.client_id` and `installed.client_secret` are the
  two values.

Then build as usual: `cargo build --release`, or `installer/make.sh` for
Setup.exe. `build.rs` bakes both values into the .exe. Google says a desktop
app's secret can't be kept secret and isn't treated as one; the sign-in is
protected by PKCE and the browser's loopback answer.

## What users see

1. They click **Continue with Google** in Sync & export, or run "Sync with
   Google Drive" from the command palette.
2. Their browser opens Google's page: "Sign in to continue to **Folio**".
3. They pick their account and click **Continue**.
4. The tab shows "You're signed in", and Folio starts syncing.

Until the app passes Google's brand verification (free, a few days; it needs
a home page and a privacy policy URL), Google first shows an "unverified app"
notice with a small "Advanced › Go to Folio" link. With only `drive.file`,
the verification is light.

A build without a client falls back to a setup dialog where a client can be
pasted, for developers only.
