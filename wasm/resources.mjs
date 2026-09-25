/** Optional fetch adapter. The renderer itself remains offline. */
export const DEFAULT_ASSET_BASE = 'https://hsjkdajbsadnmsadds.zeabur.app/Default/api/buckets/moenotes/objects/';
export function objectURL(key, base = DEFAULT_ASSET_BASE) {
  const url = new URL(base);
  if (!['https:', 'http:'].includes(url.protocol) || url.username || url.password || url.search || url.hash) throw new Error('Invalid resource base URL');
  if (!/^(Live\/MusicScore\/.*\.(json|gz)|Image\/Jacket\/.*\.png)$/.test(key) || /[\\:%?#\x00-\x1f]/.test(key) || key.split('/').some(p => !p || p === '.' || p === '..')) throw new Error('Invalid object key');
  return url.href.replace(/\/$/, '') + '/' + key.split('/').map(encodeURIComponent).join('/');
}
export function chartObjectKey(key) {
  if (!/^[A-Za-z0-9_-]+\/[A-Za-z0-9_-]+$/.test(key)) throw new Error('Expected GROUP/SCORE chart key');
  return `Live/MusicScore/${key}/${key.split('/').at(-1)}.json`;
}
export async function fetchResource(key, { base = DEFAULT_ASSET_BASE, fetch: fetcher = globalThis.fetch, signal, sha256 } = {}) {
  const url = objectURL(key, base);
  const response = await fetcher(url, { signal, credentials: 'omit' });
  if (!response.ok) throw new Error(`Resource HTTP ${response.status}`);
  if (response.headers.get('Content-Type')?.includes('text/html')) throw new Error('Expected object bytes, received a directory page');
  const limit = (key.endsWith('.png') ? 16 : 64) * 1024 * 1024;
  if (Number(response.headers.get('Content-Length')) > limit) throw new Error('Resource exceeds size limit');
  const chunks = []; let length = 0;
  if (!response.body) throw new Error('Resource response has no body');
  const reader = response.body.getReader();
  try {
    while (true) {
      const { value, done } = await reader.read(); if (done) break;
      length += value.length; if (length > limit) { await reader.cancel(); throw new Error('Resource exceeds size limit'); }
      chunks.push(value);
    }
  } finally { reader.releaseLock(); }
  const bytes = new Uint8Array(length); let offset = 0;
  for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
  const digest = [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))].map(v => v.toString(16).padStart(2, '0')).join('');
  if (sha256 && digest !== sha256.toLowerCase()) throw new Error('Resource SHA-256 mismatch');
  return { bytes, provenance: { url, object_key: key, bytes: length, sha256: digest } };
}
