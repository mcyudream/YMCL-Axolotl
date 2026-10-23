-- mc-interop v1 sites: a domain either speaks YAP (`kind = 'yap'`, the
-- historical shape) or only the cross-site `/.well-known/mc-interop-v1.json`
-- exit (`kind = 'interop'`). Interop domains cache their SiteDocument in
-- `site_json` and have no YAP capabilities/manifest.
ALTER TABLE ymcl_domains ADD COLUMN kind TEXT NOT NULL DEFAULT 'yap';
ALTER TABLE ymcl_domains ADD COLUMN site_json TEXT;
