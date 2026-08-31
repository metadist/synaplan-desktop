# `.signing/` — code-signing material (LOCAL ONLY, never committed)

This directory holds the certificates, keys, and passwords used to sign and
notarize release builds (Sprint B6 / `DC28`). **Everything here is gitignored
except this `README.md` and `secrets.env.example`.** Never commit a certificate,
key, provisioning profile, or password — not here, not anywhere.

> This repository is private, but treat signing material as if it were public
> the moment it leaves your machine. A leaked Developer ID or Authenticode key
> lets someone ship malware signed as us. Back these up **securely and
> separately** (a password manager / hardware token), and rotate on any
> suspicion of compromise.

## Layout

```
.signing/
  README.md                      # tracked (this file)
  secrets.env.example            # tracked (copy to secrets.env, fill in, never commit)
  secrets.env                    # LOCAL ONLY — passwords + IDs (gitignored)
  windows/
    authenticode.pfx             # LOCAL ONLY — OV cert export (gitignored)
  macos/
    developer_id_application.p12 # LOCAL ONLY — Developer ID Application cert (gitignored)
    AuthKey_XXXXXXXXXX.p8        # LOCAL ONLY — App Store Connect API key (gitignored)
```

Create the subfolders as you need them; they are ignored wholesale.

## What to put here

### macOS (Developer ID + notarization)

You need a **"Developer ID Application"** certificate (this is the one for apps
distributed **outside** the Mac App Store) and a way to notarize.

1. In your Apple Developer account, create/download the **Developer ID
   Application** certificate, add it to your login Keychain, then export it as a
   `.p12` (with a strong password) to `macos/developer_id_application.p12`.
2. For notarization, create an **App Store Connect API key** (Users and Access →
   Integrations → App Store Connect API, role *Developer* is enough) and save the
   `.p8` to `macos/AuthKey_XXXXXXXXXX.p8`. Note its **Key ID** and **Issuer ID**.
   (An Apple ID + app-specific password also works but the API key is preferred.)
3. Record your **Team ID** and the exact **signing identity** string
   (`Developer ID Application: Your Name (TEAMID)`) in `secrets.env`.

### Windows — Azure Trusted Signing (chosen path)

We sign Windows with **Azure Trusted Signing** (Microsoft's cloud signing), so
there is **no `.pfx` and no private key on disk** — CI authenticates to Azure
and the service signs. Set it up in the **Azure portal** (not Partner Center):

1. **Trusted Signing account** — created (`Synaplan`, RG `SynaplanSigning`). ✅
2. **Identity validation** — in the account, create an *Identity validation*
   (Organization needs a verifiable business, typically 3+ years old + D‑U‑N‑S;
   otherwise use Individual). **Approval can take hours to days — start it now.**
   The approved legal name becomes the publisher users see (SmartScreen).
3. **Certificate profile** — once validation is *Completed*, create a
   **Public Trust** certificate profile (e.g. `synaplan-desktop`) tied to that
   validation. Record its name.
4. **Signer identity for CI** — create an App registration (service principal),
   or use GitHub OIDC, and assign it the role **"Trusted Signing Certificate
   Profile Signer"** on the Trusted Signing account.
5. Record in `secrets.env` (and later as GitHub secrets): the **endpoint**
   (region URI), **account name**, **certificate profile name**, and the
   **service principal** tenant/client id (+ secret, unless OIDC).

At `DC28` the release workflow signs the built `.exe`/installer via the
[`azure/trusted-signing-action`](https://github.com/Azure/trusted-signing-action)
(or `signtool` + the Trusted Signing dlib). Nothing signing-related is stored in
this folder for the Trusted Signing path — it is all Azure RBAC + CI secrets.

## How this reaches CI (Sprint B6 / `DC28`)

The release workflow does **not** read this directory — it reads **GitHub
Actions secrets** on this private repo. This folder is where the human who owns
signing keeps the source material and generates those secrets. The mapping
(certificate → base64 → secret) is documented in `secrets.env.example`, and the
release workflow that consumes them is added in `DC28`.

Reference: the platform plan's signing section
(`synaplan/_devextras/planning/20260829-desktop-agent-client/13_cross_platform.md` §9)
and `docs/PLATFORMS.md`.
