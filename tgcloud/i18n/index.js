/*
 * Personal and Educational Free License (PEF License / PEFL)
 * Version 1.0 - 2026-05-26
 * Copyright (c) 2026 Rubens Rainelli (rubens@rainelli.it)
 *
 * Personal and educational free use permitted.
 * Corporate, business or enterprise use prohibited without written authorization and fee.
 */

import en from '../../locales/en.json' with { type: 'json' };
import it from '../../locales/it.json' with { type: 'json' };

const CATALOGS = {
  en,
  it,
};

/**
 * Resolves the best matching language code from Telegram's `user.language_code`.
 * Supports Italian ('it') and defaults to English ('en') if language is not available.
 * Contributors can add new JSON files in `locales/<code>.json` and register them in `CATALOGS`.
 */
export function resolveLocale(languageCode) {
  if (!languageCode || typeof languageCode !== 'string') return 'en';
  const normalized = languageCode.trim().toLowerCase().split('-')[0].split('_')[0];
  return CATALOGS[normalized] ? normalized : 'en';
}

/**
 * Translates a dot-separated key (e.g., 'bot.welcome_title') with `{param}` interpolation.
 */
export function t(languageCode, keyPath, params = {}) {
  const lang = resolveLocale(languageCode);
  const parts = keyPath.split('.');
  let current = CATALOGS[lang];
  for (const p of parts) {
    current = current?.[p];
  }
  if (typeof current !== 'string') {
    current = CATALOGS.en;
    for (const p of parts) {
      current = current?.[p];
    }
  }
  if (typeof current !== 'string') return keyPath;

  return current.replace(/\{(\w+)\}/g, (_, k) =>
    params[k] !== undefined && params[k] !== null ? String(params[k]) : `{${k}}`
  );
}
