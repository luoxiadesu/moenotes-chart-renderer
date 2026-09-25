export const DEFAULT_ASSET_BASE: string;
export function objectURL(key: string, base?: string): string;
export function chartObjectKey(key: string): string;
export interface ResourceOptions {base?: string; fetch?: typeof globalThis.fetch; signal?: AbortSignal; sha256?: string;}
export interface Resource {bytes: Uint8Array; provenance: {url: string; object_key: string; bytes: number; sha256: string};}
export function fetchResource(key: string, options?: ResourceOptions): Promise<Resource>;
