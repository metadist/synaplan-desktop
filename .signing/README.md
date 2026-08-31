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

### Windows (Authenticode)

You need an **OV or EV** code-signing certificate.

- **OV (file-based):** export the certificate + private key as a `.pfx`/`.p12`
  (with a password) to `windows/authenticode.pfx`. Put the password and the
  cert **thumbprint** in `secrets.env`.
- **EV or cloud signing (token / HSM):** EV certificates are usually
  non-exportable (hardware token) or issued through a cloud service
  (Azure Trusted Signing, DigiCert KeyLocker, SSL.com eSigner). In that case
  there is no `.pfx` to store — record the **service account references** in
  `secrets.env` instead, and we wire the cloud signer into CI at `DC28`. EV
  avoids the SmartScreen reputation ramp, so it is the better choice if you have
  it.

## How this reaches CI (Sprint B6 / `DC28`)

The release workflow does **not** read this directory — it reads **GitHub
Actions secrets** on this private repo. This folder is where the human who owns
signing keeps the source material and generates those secrets. The mapping
(certificate → base64 → secret) is documented in `secrets.env.example`, and the
release workflow that consumes them is added in `DC28`.

Reference: the platform plan's signing section
(`synaplan/_devextras/planning/20260829-desktop-agent-client/13_cross_platform.md` §9)
and `docs/PLATFORMS.md`.
