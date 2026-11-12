// Encontrar el PC por su identificador: primero las direcciones guardadas (casa, Tailscale), después se busca en la
// red local (UDP) y se guarda la que responda. En cada `hello` se renuevan las direcciones que da el PC.
import { saveServer } from './cache';
import { orderAddrs, splitAddr } from './pairing';
import { hello, type Hello, type ServerCfg } from './source';
import { Streaming } from './streaming.svelte';

const sameAddrs = (a?: string[], b?: string[]) => JSON.stringify(a ?? []) === JSON.stringify(b ?? []);

/** `s` con la dirección que ha respondido y las direcciones y el nombre que dice el PC; se guarda si cambió algo. */
function adopt(s: ServerCfg, host: string, port: number, h: Hello): ServerCfg {
  const addrs = h.addrs.length ? orderAddrs(h.addrs) : s.addrs;
  const next: ServerCfg = { ...s, host, port, id: s.id || h.id || undefined, name: h.name || s.name, addrs };
  if (next.host !== s.host || next.port !== s.port || next.id !== s.id || next.name !== s.name || !sameAddrs(next.addrs, s.addrs)) saveServer(next);
  return next;
}

async function tryAddr(s: ServerCfg, host: string, port: number): Promise<ServerCfg | null> {
  try {
    const h = await hello(host, port, 1500);
    if (s.id && h.id && h.id !== s.id) return null; // en esa dirección hay otro PC
    return adopt(s, host, port, h);
  } catch { return null; }
}

export async function relocate(s: ServerCfg): Promise<ServerCfg> {
  const here = await tryAddr(s, s.host, s.port);
  if (here) return here;
  for (const a of orderAddrs(s.addrs ?? [])) {
    const p = splitAddr(a);
    if (!p || (p.host === s.host && p.port === s.port)) continue;
    const m = await tryAddr(s, p.host, p.port);
    if (m) return m;
  }
  if (!s.id) return s;
  try {
    const { found } = await Streaming.discover({ timeoutMs: 2000 });
    const f = found.find((x) => x.id === s.id);
    if (f) {
      const m = await tryAddr(s, f.host, f.port);
      if (m) return m;
    }
  } catch { /* sin red o fuera de Android */ }
  return s;
}
